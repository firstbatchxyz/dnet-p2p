use std::time::Duration;

use clap::Subcommand;

use clap::Parser;
use dllmd::DLLMP2P;
use libp2p::identity::Keypair;
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
    tokio::spawn(async move { dllmd::wait_for_termination(cancellation_clone).await });

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
