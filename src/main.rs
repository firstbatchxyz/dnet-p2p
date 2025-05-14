use std::{env, sync::Arc};

use clap::{Parser, Subcommand};
use dnet_p2p::{DnetService, ServiceProperties};
use gethostname::gethostname;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

#[derive(Subcommand)]
enum Commands {
    /// Starts the worker service, actively listening to a manager.
    Worker,
    /// Start the manager service, which will monitor the workers.
    Manager,
}

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    env_logger::builder()
        .format_timestamp_millis()
        .filter(None, log::LevelFilter::Off)
        .filter_module("dnet_p2p", log::LevelFilter::Debug)
        .parse_default_env()
        .init();

    let cancellation = CancellationToken::new();
    let properties = Arc::new(RwLock::new(ServiceProperties::default()));

    // spawn a task to listen for termination signals
    let cancellation_clone = cancellation.clone();
    let handle_for_cancellation =
        tokio::spawn(async move { wait_for_termination(cancellation_clone).await });

    let args = Cli::parse();
    let mut mdns = DnetMDNSDameon::new(cancellation.clone(), properties.clone());
    match args.command {
        Commands::Worker => {
            // create a service that binds to a random port
            let mut service = DnetService::new(cancellation, properties).await?;
            let port = service.get_port()?;

            // start listening for incoming connections on a new task
            let handle_for_service = tokio::spawn(async move {
                service.start().await.unwrap();
            });

            // register the service with mDNS
            let instance_name = env::var("INSTANCE_NAME").unwrap_or("TODO-random".to_string());
            let hostname = env::var("HOSTNAME").unwrap_or_else(|_| {
                gethostname()
                    .into_string()
                    .unwrap_or("TODO-random".to_string())
            });
            if let Err(e) = mdns.register(&instance_name, &hostname, port).await {
                log::error!("Failed to register mDNS service: {}", e);
            };

            log::info!("Aborting service...");
            if let Err(e) = handle_for_service.await {
                log::error!("Error while waiting for service: {}", e);
            }
        }
        Commands::Manager => {
            mdns.browse().await?;
        }
    };

    if let Err(e) = handle_for_cancellation.await {
        log::error!("Error while waiting for handles: {}", e);
    }

    log::info!("Bye!\n");

    Ok(())
}

/// Waits for various termination signals, and cancels the given token when the signal is received.
///
/// Handles Unix and Windows [target families](https://doc.rust-lang.org/reference/conditional-compilation.html#target_family).
async fn wait_for_termination(cancellation: CancellationToken) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate())?; // Docker sends SIGTERM
        let mut sigint = signal(SignalKind::interrupt())?; // Ctrl+C sends SIGINT
        tokio::select! {
            _ = sigterm.recv() => log::warn!("Recieved SIGTERM"),
            _ = sigint.recv() => log::warn!("Recieved SIGINT"),
            _ = cancellation.cancelled() => {
                // no need to wait if cancelled anyways
                // although this is not likely to happen
                return Ok(());
            }
        };

        cancellation.cancel();
    }

    #[cfg(windows)]
    {
        use tokio::signal::windows;

        // https://learn.microsoft.com/en-us/windows/console/handlerroutine
        let mut signal_c = windows::ctrl_c()?;
        let mut signal_break = windows::ctrl_break()?;
        let mut signal_close = windows::ctrl_close()?;
        let mut signal_shutdown = windows::ctrl_shutdown()?;

        tokio::select! {
            _ = signal_c.recv() => log::warn!("Received CTRL_C"),
            _ = signal_break.recv() => log::warn!("Received CTRL_BREAK"),
            _ = signal_close.recv() => log::warn!("Received CTRL_CLOSE"),
            _ = signal_shutdown.recv() => log::warn!("Received CTRL_SHUTDOWN"),
            _ = cancellation.cancelled() => {
                // no need to wait if cancelled anyways
                // although this is not likely to happen
                return Ok(());
            }
        };

        cancellation.cancel();
    }

    #[cfg(not(any(unix, windows)))]
    {
        log::error!("No signal handling for this platform: {}", env::consts::OS);
        cancellation.cancel();
    }

    Ok(())
}
