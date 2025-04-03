use clap::{Parser, Subcommand};
use dllmd::DLLMP2P;
use libp2p::identity::Keypair;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

#[derive(Subcommand)]
pub enum Commands {
    /// Check existing topology.
    Topo,
    /// Run the dLLM daemon.
    Daemon,
}

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() {
    env_logger::builder()
        .format_timestamp_millis()
        .filter(None, log::LevelFilter::Off)
        .filter_module("dllmd", log::LevelFilter::Debug)
        .parse_default_env()
        .init();

    let cancellation = CancellationToken::new();

    // spawn a task to listen for termination signals
    let cancellation_clone = cancellation.clone();
    tokio::spawn(async move { wait_for_termination(cancellation_clone).await });

    let args = Cli::parse();
    match args.command {
        Commands::Daemon => {
            DLLMP2P::new(Keypair::generate_ed25519())
                .unwrap()
                .run_daemon(cancellation, None)
                .await;
        }

        Commands::Topo => {
            let keypair = Keypair::generate_ed25519(); // see TOPO
            DLLMP2P::new(keypair)
                .unwrap()
                .run_topo(cancellation, Duration::from_secs(5))
                .await;
        }
    }
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
