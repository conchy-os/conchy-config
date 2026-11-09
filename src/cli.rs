use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "conchy-config",
    author = "Conchy OS Team",
    version,
    about = "Unified reactive KDL configuration daemon, library, and CLI for Conchy OS",
    long_about = "conchy-config provides a centralized, reactive configuration system for Conchy OS.\n\
                  It reads and writes human-friendly KDL documents, maintains an active D-Bus bus (org.conchy.Config),\n\
                  monitors filesystem changes via Inotify to support live updates from Vim/Neovim, and allows\n\
                  instant live preview for GUI configurators and scripts."
)]
pub struct Cli {
    /// Custom configuration directory [default: ~/.config/conchy]
    #[arg(short = 'C', long, global = true, value_name = "DIR")]
    pub config_dir: Option<PathBuf>,

    /// Log level filter (trace, debug, info, warn, error)
    #[arg(short, long, global = true, env = "CONCHY_LOG", default_value = "info")]
    pub log_level: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Read a configuration property value
    Get {
        /// Channel name (e.g. "wm", "session", "shell")
        channel: String,
        /// Dot-path to property (e.g. "layout.gaps", "panel.height")
        key: String,
    },

    /// Set a configuration property value and broadcast change via D-Bus
    Set {
        /// Channel name (e.g. "wm", "session", "shell")
        channel: String,
        /// Dot-path to property (e.g. "layout.gaps", "panel.height")
        key: String,
        /// New value (automatically parsed as bool, int, float, or string)
        value: String,
    },

    /// List all properties defined in a channel
    List {
        /// Channel name (e.g. "wm", "session", "shell")
        channel: String,
    },

    /// Listen to live stream of property change events via D-Bus
    Watch {
        /// Optional channel filter
        channel: Option<String>,
    },

    /// Run the background D-Bus configuration daemon and Inotify watcher
    Daemon,

    /// Generate default configuration files in ~/.config/conchy/
    Init,
}
