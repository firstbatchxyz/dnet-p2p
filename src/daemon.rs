use std::time::Duration;

use mdns_sd::{ServiceDaemon, ServiceEvent};

const DAEMON_SERVICE: &str = "_dllmd._tcp.local.";

/// Number of seconds between refreshing for diagnostic prints.
const DAEMON_DISCOVERY_TIMEOUT_SEC: Duration = Duration::from_secs(2);

pub async fn run_daemon(hostname: String) {
    let mut ticker = tokio::time::interval(DAEMON_DISCOVERY_TIMEOUT_SEC);
    ticker.tick().await; // move one tick

    let mdns = ServiceDaemon::new().unwrap();

    // TODO: check your own hostname if it exists
    if let Ok(existing) = mdns.resolve_hostname(&hostname, None).unwrap().recv() {
        println!("Hostname already exists: {:?}", existing);
        return;
    }

    // TODO: use `unregister` instead
    let receiver = mdns.browse(DAEMON_SERVICE).expect("Failed to browse");
    let mut master_exists = false;
    loop {
        tokio::select! {
          _ = ticker.tick() => {
              log::info!("Did not detect anyone. Shutting down daemon.");
              if let Ok(event) = mdns.shutdown().unwrap().recv() {
                  println!("Daemon shutdown: {:?}", event);
              }
              break;
          },
          event_res = receiver.recv_async() => {
              match event_res {
                Err(e) => {
                  log::error!("Error receiving event: {:?}", e);
                }
                Ok(event) => {
                  match event {
                      ServiceEvent::ServiceResolved(info) => {
                          println!(
                              "Resolved a new service: {}\n host: {}\n port: {}",
                              info.get_fullname(),
                              info.get_hostname(),
                              info.get_port(),
                          );
                          for addr in info.get_addresses().iter() {
                              println!(" Address: {}", addr);
                          }
                          for prop in info.get_properties().iter() {
                              println!(" Property: {}", prop);
                          }
                          master_exists = true;
                          break;
                      }
                      other_event => {
                          log::debug!("{:?}", other_event);
                      }
                  }
              }
            };
          }
        }
    }

    log::info!("Master exists: {}", master_exists);
    if let Ok(event) = mdns.shutdown().unwrap().recv() {
        println!("Daemon shutdown: {:?}", event);
    }
}
