use anyhow::{Context, Result};
use kdl::KdlDocument;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::{debug, info};

use crate::ast;
use crate::schema::{default_session_kdl, default_shell_kdl, default_wm_kdl};

#[derive(Clone)]
pub struct ConfigStore {
    base_dir: PathBuf,
    // Tracks timestamps of recent programmatic writes to ignore in the Inotify watcher
    recent_writes: Arc<Mutex<HashMap<String, Instant>>>,
}

impl ConfigStore {
    pub fn new(custom_dir: Option<PathBuf>) -> Result<Self> {
        let base_dir = if let Some(dir) = custom_dir {
            dir
        } else if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
            PathBuf::from(xdg).join("conchy")
        } else if let Ok(home) = env::var("HOME") {
            PathBuf::from(home).join(".config").join("conchy")
        } else {
            PathBuf::from("/etc/conchy")
        };

        if !base_dir.exists() {
            fs::create_dir_all(&base_dir)
                .with_context(|| format!("Failed to create config directory at {:?}", base_dir))?;
        }

        let store = Self {
            base_dir,
            recent_writes: Arc::new(Mutex::new(HashMap::new())),
        };

        store.ensure_defaults()?;
        Ok(store)
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Resolves the file path for a channel name (e.g. "session" -> ".../session.kdl")
    pub fn channel_path(&self, channel: &str) -> PathBuf {
        self.base_dir.join(format!("{}.kdl", channel))
    }

    /// Ensures standard default KDL files exist
    pub fn ensure_defaults(&self) -> Result<()> {
        let defaults = [
            ("session", default_session_kdl()),
            ("wm", default_wm_kdl()),
            ("shell", default_shell_kdl()),
        ];

        for (channel, content) in defaults {
            let path = self.channel_path(channel);
            if !path.exists() {
                info!("Creating default configuration: {:?}", path);
                fs::write(&path, content)
                    .with_context(|| format!("Failed to write default config to {:?}", path))?;
            }
        }

        Ok(())
    }

    /// Loads and parses the KdlDocument for a channel
    pub fn load_document(&self, channel: &str) -> Result<KdlDocument> {
        let path = self.channel_path(channel);
        if !path.exists() {
            return Ok(KdlDocument::new());
        }

        let raw = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file {:?}", path))?;
        let doc: KdlDocument = raw
            .parse()
            .with_context(|| format!("Failed to parse KDL syntax in {:?}", path))?;

        Ok(doc)
    }

    /// Saves a KdlDocument atomically to disk
    pub fn save_document(&self, channel: &str, doc: &KdlDocument) -> Result<()> {
        let path = self.channel_path(channel);
        let content = doc.to_string();

        // Record programmatic write timestamp to debounce in watcher
        {
            let mut writes = self.recent_writes.lock().unwrap();
            writes.insert(channel.to_string(), Instant::now());
        }

        // Write to temporary file first in the same directory, then rename (atomic)
        let temp_path = self.base_dir.join(format!(".{}.tmp", channel));
        fs::write(&temp_path, content)
            .with_context(|| format!("Failed to write temp file {:?}", temp_path))?;
        fs::rename(&temp_path, &path)
            .with_context(|| format!("Failed to atomically rename {:?} to {:?}", temp_path, path))?;

        debug!("Saved updated configuration for channel '{}'", channel);
        Ok(())
    }

    /// Reads a single property via dot-path
    pub fn get(&self, channel: &str, key: &str) -> Result<Option<String>> {
        let doc = self.load_document(channel)?;
        Ok(ast::get_value(&doc, key))
    }

    /// Sets a property via dot-path and persists it atomically
    pub fn set(&self, channel: &str, key: &str, value: &str) -> Result<()> {
        let mut doc = self.load_document(channel)?;
        ast::set_value(&mut doc, key, value)?;
        self.save_document(channel, &doc)?;
        info!("Updated [{}] {} = {}", channel, key, value);
        Ok(())
    }

    /// Lists all properties for a channel
    pub fn list(&self, channel: &str) -> Result<Vec<(String, String)>> {
        let doc = self.load_document(channel)?;
        Ok(ast::list_values(&doc))
    }

    /// Checks if a file modification was programmatic (within last 300ms)
    pub fn is_recent_programmatic_write(&self, channel: &str) -> bool {
        let mut writes = self.recent_writes.lock().unwrap();
        if let Some(&time) = writes.get(channel) {
            if time.elapsed().as_millis() < 400 {
                return true;
            }
        }
        writes.remove(channel);
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_store_lifecycle() {
        let tmp = tempdir().unwrap();
        let store = ConfigStore::new(Some(tmp.path().to_path_buf())).unwrap();

        // Defaults should be created
        assert!(store.channel_path("session").exists());
        assert!(store.channel_path("wm").exists());
        assert!(store.channel_path("shell").exists());

        // Read default property
        let val = store.get("session", "session.socket-wait-timeout").unwrap();
        assert_eq!(val, Some("10".to_string()));

        // Mutate property
        store.set("session", "session.socket-wait-timeout", "25").unwrap();
        let updated = store.get("session", "session.socket-wait-timeout").unwrap();
        assert_eq!(updated, Some("25".to_string()));

        // Check file on disk
        let raw = fs::read_to_string(store.channel_path("session")).unwrap();
        assert!(raw.contains("25"));
    }
}
