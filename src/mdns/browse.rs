use mdns_sd::{ServiceDaemon, ServiceEvent};
use tokio_util::sync::CancellationToken;

use crate::mdns::DNET_SERVICE_TYPE;

/// Browses for mDNS services and prints the resolved service information.
pub async fn browse_mdns(cancellation: CancellationToken) -> eyre::Result<()> {
    let mdns = ServiceDaemon::new()?;

    // brwose the service
    let receiver = mdns.browse(DNET_SERVICE_TYPE).expect("failed to browse");
    loop {
        tokio::select! {
          _ = cancellation.cancelled() => break,
          event_res = receiver.recv_async() => {
              match event_res {
                Err(e) => {
                  log::error!("Error receiving event: {:?}", e);
                }
                Ok(event) => {
                  match event {
                      ServiceEvent::ServiceResolved(info) => {
                          if info.get_fullname().ends_with(DNET_SERVICE_TYPE) {
                              println!(
                                  "Resolved a new service: {}\n host: {}\n port: {}",
                                  info.get_fullname(),
                                  info.get_hostname(),
                                  info.get_port(),
                              );
                              for addr in info.get_addresses().iter() {
                                  println!(" Address: {}", addr);
                              }

                              // txt records
                              for prop in info.get_properties().iter() {
                                  println!(" Property: {}", prop);
                              }

                          } else {
                              log::debug!("Ignoring service {}", info.get_fullname());
                          }
                      },
                      ServiceEvent::ServiceRemoved(service_name, fullname) => {
                          println!("Service {service_name} removed: {fullname}");
                      },
                      other_event => {
                          log::trace!("{:?}", other_event);
                      }
                  }
              }
            };
          }
        }
    }

    log::info!("Shutting down daemon.");
    mdns.stop_browse(DNET_SERVICE_TYPE).unwrap();
    if let Ok(status) = mdns.shutdown().unwrap().recv() {
        println!("Daemon status: {:?}", status);
    }

    Ok(())
}
