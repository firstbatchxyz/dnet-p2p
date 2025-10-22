use clap::Parser;
use dnet_p2p::DnetService;

use tokio_util::sync::CancellationToken;
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Run as manager instead of worker
    #[arg(short = 'm', long = "manager", default_value_t = false)]
    is_manager: bool,
    /// Run in passive mode (monitor only, don't register to mDNS)
    #[arg(short = 'p', long = "passive", default_value_t = false)]
    is_passive: bool,
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

    // spawn a task to listen for termination signals
    let cancellation_clone = cancellation.clone();
    let handle_for_cancellation =
        tokio::spawn(async move { wait_for_termination(cancellation_clone).await });

    let args = Cli::parse();

    // get time just for the sake of having a unique instance name
    let instance_name = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        .to_string();

    // create the service with UDP discovery
    let mut service = DnetService::new(
        cancellation,
        instance_name,
        8080,  // dummy HTTP server port
        50501, // dummy shard port
        args.is_manager,
        args.is_passive,
    )
    .await?;
    let handle_for_service = tokio::spawn(async move { service.start().await });

    if let Err(err) = handle_for_service.await {
        log::error!("Error while waiting for service: {err}");
    }
    if let Err(err) = handle_for_cancellation.await {
        log::error!("Error while waiting for handles: {err}");
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
