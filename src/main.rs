use clap::Subcommand;

use clap::Parser;
use dllmd::query_services;
use dllmd::register_service;
use gethostname::gethostname;

/// [DNS-SD meta query](https://www.rfc-editor.org/rfc/rfc6763.html#section-9)
const DNS_SD_SERVICE: &str = "_services._dns-sd._udp.local.";

#[derive(Subcommand)]
pub enum Commands {
    /// Query existing services.
    Query {
        /// Name of the service to query.
        /// Must end with `._udp.local.` or `._tcp.local.`.
        #[arg(default_value = DNS_SD_SERVICE)]
        service: String,
    },
    /// Become a service.
    Register {
        /// Must end with `._udp.local.` or `._tcp.local.`.
        #[arg()]
        service: String,
        /// Name of the service instance.
        #[arg()]
        instance: String,

        #[arg(long)]
        /// Optional hostname of the service, defaults to machine host name.
        hostname: Option<String>,

        /// Whether to unregister the service after a while.
        #[arg(short, long)]
        unregister: bool,

        /// Whether to disable IPv6.
        #[arg(short, long)]
        disable_ipv6: bool,
    },
    /// Run the dLLM daemon.
    Daemon {
        #[arg(long)]
        /// Optional hostname of the service, defaults to machine host name.
        hostname: Option<String>,
    },
}

#[derive(Parser)]
#[command(name = env!("CARGO_PKG_NAME"), version, about)]
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
        .filter_module("mdns_sd", log::LevelFilter::Info) // enable Debug
        .parse_default_env()
        .init();

    let args = Cli::parse();
    match args.command {
        Commands::Query { service } => {
            query_services(service);
        }
        Commands::Register {
            service,
            instance,
            hostname,
            unregister,
            disable_ipv6,
        } => {
            let hostname = hostname.unwrap_or_else(|| gethostname().to_string_lossy().to_string());
            register_service(service, instance, hostname, unregister, disable_ipv6);
        }
        Commands::Daemon { hostname } => {
            println!("hostname: {:?}", gethostname());
            // TODO: !!!
            let hostname = hostname.unwrap_or_else(|| {
                gethostname()
                    .to_string_lossy()
                    .to_string()
                    .replace(".local", ".local.")
            });
            dllmd::run_daemon(hostname).await;
        }
    }
}
