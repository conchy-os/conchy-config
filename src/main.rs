use anyhow::{Context, Result};
use clap::Parser;
use std::process::ExitCode;
use tracing::warn;
use tracing_subscriber::EnvFilter;
use zbus::Connection;

use conchy_config::bus::ConfigClient;
use conchy_config::cli::{Cli, Commands};
use conchy_config::daemon::Daemon;
use conchy_config::store::ConfigStore;
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> ExitCode {
    let args = Cli::parse();

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&args.log_level));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .init();

    match run_command(args).await {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {:#}", e);
            ExitCode::FAILURE
        }
    }
}

async fn run_command(args: Cli) -> Result<()> {
    match args.command {
        Commands::Daemon => {
            Daemon::run(args.config_dir).await?;
        }
        Commands::Init => {
            let store = ConfigStore::new(args.config_dir)?;
            store.ensure_defaults()?;
            println!(
                "Initialized default Conchy OS KDL configurations in {:?}",
                store.base_dir()
            );
        }
        Commands::Get { channel, key } => {
            // Try D-Bus first, fallback to disk store
            if let Ok(conn) = Connection::session().await {
                if let Ok(client) = ConfigClient::new(&conn).await {
                    if let Ok(val) = client.get(&channel, &key).await {
                        println!("{}", val);
                        return Ok(());
                    }
                }
            }

            // Fallback directly to disk
            let store = ConfigStore::new(args.config_dir)?;
            match store.get(&channel, &key)? {
                Some(val) => println!("{}", val),
                None => {
                    eprintln!("Property '{}' not found in channel '{}'", key, channel);
                    std::process::exit(1);
                }
            }
        }
        Commands::Set {
            channel,
            key,
            value,
        } => {
            // Try D-Bus first so listeners receive the signal immediately
            if let Ok(conn) = Connection::session().await {
                if let Ok(client) = ConfigClient::new(&conn).await {
                    if client.set(&channel, &key, &value).await.is_ok() {
                        println!("Updated [{}] {} = {} (via D-Bus)", channel, key, value);
                        return Ok(());
                    }
                }
            }

            // Fallback directly to disk
            let store = ConfigStore::new(args.config_dir)?;
            store.set(&channel, &key, &value)?;
            println!("Updated [{}] {} = {} (direct file write)", channel, key, value);
        }
        Commands::List { channel } => {
            let store = ConfigStore::new(args.config_dir)?;
            let entries = store.list(&channel)?;
            for (k, v) in entries {
                println!("{} = {}", k, v);
            }
        }
        Commands::Watch { channel: filter_ch } => {
            let conn = Connection::session()
                .await
                .context("Failed to connect to D-Bus session bus")?;
            let client = ConfigClient::new(&conn)
                .await
                .context("Could not connect to org.conchy.Config D-Bus service. Is the daemon running?")?;

            println!("Listening for live configuration changes via D-Bus...");
            let mut stream = client.receive_property_changed().await?;

            while let Some(signal) = stream.next().await {
                let args = match signal.args() {
                    Ok(a) => a,
                    Err(e) => {
                        warn!("Error decoding signal args: {}", e);
                        continue;
                    }
                };

                if let Some(ref target) = filter_ch {
                    if &args.channel != target {
                        continue;
                    }
                }

                println!("[{}] {} = {}", args.channel, args.key, args.value);
            }
        }
    }

    Ok(())
}
