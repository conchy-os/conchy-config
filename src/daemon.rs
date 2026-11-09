use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::signal::unix::{signal, SignalKind};
use tracing::info;
use zbus::connection::Builder as ConnectionBuilder;

use crate::bus::{ConfigServer, DBUS_OBJECT_PATH, DBUS_SERVICE_NAME};
use crate::store::ConfigStore;
use crate::watcher::FileWatcher;

pub struct Daemon;

impl Daemon {
    pub async fn run(custom_dir: Option<PathBuf>) -> Result<()> {
        info!("Starting conchy-config D-Bus daemon (org.conchy.Config)...");

        let store = ConfigStore::new(custom_dir)?;
        info!("Config directory resolved to: {:?}", store.base_dir());

        let server = ConfigServer::new(store.clone());

        // Connect to session D-Bus and register service
        let connection = ConnectionBuilder::session()?
            .name(DBUS_SERVICE_NAME)
            .context("Failed to request D-Bus name org.conchy.Config (maybe already running?)")?
            .serve_at(DBUS_OBJECT_PATH, server)?
            .build()
            .await
            .context("Failed to build D-Bus connection")?;

        info!("D-Bus service '{}' registered successfully at '{}'", DBUS_SERVICE_NAME, DBUS_OBJECT_PATH);

        // Start file watcher for live manual edits
        let watcher = Arc::new(FileWatcher::new(store, connection.clone()));
        watcher.start().await.context("Failed to start Inotify file watcher")?;

        // Wait for shutdown signals
        let mut sigterm = signal(SignalKind::terminate()).context("Failed to hook SIGTERM")?;
        let mut sigint = signal(SignalKind::interrupt()).context("Failed to hook SIGINT")?;

        info!("conchy-config daemon is running and ready to serve requests.");

        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM, shutting down daemon...");
            }
            _ = sigint.recv() => {
                info!("Received SIGINT, shutting down daemon...");
            }
        }

        info!("conchy-config daemon stopped cleanly.");
        Ok(())
    }
}
