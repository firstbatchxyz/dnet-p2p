// TODO: add dnet.h stuff here

/// Enables logging for `dnet` while respecting
/// the `RUST_LOG` environment variable.
///
/// ---
///
/// C/C++ declaration:
/// ```c
/// extern void dnet_enable_logs(void);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn dnet_enable_logs() {
    if let Err(err) = env_logger::builder()
        .filter(None, log::LevelFilter::Off)
        .filter_module("dnet_p2p", log::LevelFilter::Info)
        .parse_default_env() // reads RUST_LOG variable
        .try_init()
    {
        eprintln!("Could not enable logs: {err}");
    }
}
