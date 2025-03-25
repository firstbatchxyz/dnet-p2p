use clap::Subcommand;

use clap::Parser;
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
    let cancellation_clone = cancellation.clone();
    tokio::spawn(async move { dllmd::wait_for_termination(cancellation_clone).await });

    let args = Cli::parse();
    match args.command {
        Commands::Topo => {
            dllmd::get_topology(cancellation).await;
        }

        Commands::Daemon => {
            dllmd::run_daemon(cancellation).await;
        }
    }
}
