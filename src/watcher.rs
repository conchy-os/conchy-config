use anyhow::{Context, Result};
use notify::{Config as NotifyConfig, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{debug, info, warn};
use zbus::Connection;

use crate::bus::{ConfigServer, DBUS_OBJECT_PATH};
use crate::store::ConfigStore;

pub struct FileWatcher {
    store: ConfigStore,
    dbus_conn: Connection,
    cache: Arc<Mutex<HashMap<String, HashMap<String, String>>>>,
}

impl FileWatcher {
    pub fn new(store: ConfigStore, dbus_conn: Connection) -> Self {
        Self {
            store,
            dbus_conn,
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Initializes in-memory snapshot of all existing channels
    pub async fn initialize_snapshot(&self) {
        let channels = ["session", "wm", "shell"];
        let mut cache = self.cache.lock().await;

        for ch in channels {
            if let Ok(entries) = self.store.list(ch) {
                let map: HashMap<String, String> = entries.into_iter().collect();
                cache.insert(ch.to_string(), map);
            }
        }
    }

    /// Starts watching base_dir for *.kdl changes in a background tokio task
    pub async fn start(self: Arc<Self>) -> Result<()> {
        self.initialize_snapshot().await;

        let base_dir = self.store.base_dir().to_path_buf();
        let (tx, rx) = channel::<notify::Result<Event>>();

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            NotifyConfig::default().with_poll_interval(Duration::from_millis(200)),
        )
        .context("Failed to initialize notify watcher")?;

        watcher
            .watch(&base_dir, RecursiveMode::NonRecursive)
            .with_context(|| format!("Failed to watch directory {:?}", base_dir))?;

        info!("Inotify watcher active on {:?}", base_dir);

        let watcher_self = Arc::clone(&self);
        tokio::spawn(async move {
            // Keep watcher alive in task
            let _watcher = watcher;

            while let Ok(event_res) = rx.recv() {
                match event_res {
                    Ok(event) => {
                        watcher_self.handle_event(event).await;
                    }
                    Err(e) => {
                        warn!("Notify watcher error: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    async fn handle_event(&self, event: Event) {
        // We only care about data modifications or create/rename
        match event.kind {
            EventKind::Modify(_) | EventKind::Create(_) => {}
            _ => return,
        }

        for path in event.paths {
            let file_name = match path.file_name().and_then(|f| f.to_str()) {
                Some(n) => n,
                None => continue,
            };

            // Ignore temporary files like .wm.tmp
            if file_name.starts_with('.') || !file_name.ends_with(".kdl") {
                continue;
            }

            let channel = file_name.trim_end_matches(".kdl").to_string();

            // Skip programmatic writes performed by ConfigStore itself
            if self.store.is_recent_programmatic_write(&channel) {
                debug!("Skipping programmatic write notification for [{}]", channel);
                continue;
            }

            // Debounce briefly for editor multi-writes
            sleep(Duration::from_millis(100)).await;

            self.diff_and_notify(&channel).await;
        }
    }

    async fn diff_and_notify(&self, channel: &str) {
        let current_entries = match self.store.list(channel) {
            Ok(entries) => entries,
            Err(e) => {
                debug!("Could not read entries for channel [{}]: {}", channel, e);
                return;
            }
        };

        let current_map: HashMap<String, String> = current_entries.into_iter().collect();

        let mut cache = self.cache.lock().await;
        let previous_map = cache.entry(channel.to_string()).or_default();

        let mut changed_keys = Vec::new();

        // Detect changed or added keys
        for (k, new_val) in &current_map {
            if previous_map.get(k) != Some(new_val) {
                changed_keys.push((k.clone(), new_val.clone()));
            }
        }

        // Update cache
        *previous_map = current_map;
        drop(cache);

        if changed_keys.is_empty() {
            return;
        }

        info!(
            "Detected manual file edit in [{}.kdl]: {} properties changed",
            channel,
            changed_keys.len()
        );

        // Broadcast PropertyChanged signal for each modified property
        if let Ok(object_server) = self.dbus_conn.object_server().interface::<_, ConfigServer>(DBUS_OBJECT_PATH).await {
            let emitter = object_server.signal_emitter();
            for (k, v) in changed_keys {
                info!("Broadcasting manual change via D-Bus: [{}] {} = {}", channel, k, v);
                let _ = ConfigServer::property_changed(&emitter, channel, &k, &v).await;
            }
        }
    }
}
