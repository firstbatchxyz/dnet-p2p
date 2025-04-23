use mdns_sd::{DaemonEvent, IntoTxtProperties, ServiceDaemon, ServiceInfo, UnregisterStatus};

use tokio_util::sync::CancellationToken;

use crate::mdns::DNET_SERVICE_TYPE;

pub async fn register_mdns(
    instance_name: String,
    hostname: String,
    cancellation: CancellationToken,
    port: u16,
    properties: impl IntoTxtProperties,
) -> eyre::Result<()> {
    let mdns = ServiceDaemon::new()?;
    let service_hostname = format!("{}.local.", hostname);

    // check your own hostname if it exists
    // if let Ok(HostnameResolutionEvent::AddressesFound(existing_hostname, addrs)) =
    //     mdns.resolve_hostname(&service_hostname, None)?.recv()
    // {
    //     println!(
    //         "Host {} already exists at {:?}, unregistering.",
    //         existing_hostname, addrs
    //     );
    //     mdns.stop_resolve_hostname(&service_hostname).unwrap();
    // } else {
    //     mdns.stop_resolve_hostname(&service_hostname).unwrap();

    // register your own hostname
    println!("Registering hostname {}", hostname);
    let my_addrs = "";

    // Register a service.
    let service_type = DNET_SERVICE_TYPE;
    let service_info = ServiceInfo::new(
        service_type,
        &instance_name,
        &service_hostname,
        my_addrs,
        port,
        properties,
    )
    .expect("valid service info")
    .enable_addr_auto();

    // Optionally, we can monitor the daemon events.
    let monitor = mdns.monitor().expect("Failed to monitor the daemon");
    let service_fullname = service_info.get_fullname().to_string();
    mdns.register(service_info)
        .expect("Failed to register mDNS service");

    println!("Registered service {}.{}", &instance_name, &service_type);

    loop {
        tokio::select! {
          // monitor mdns events
          event_opt = monitor.recv_async() => {
              match event_opt {
                  Ok(event) => {
                      println!("Daemon event: {:?}", &event);
                      if let DaemonEvent::Error(e) = event {
                          println!("Failed: {}", e);
                      }
                  },
                  Err(e) => {
                      log::error!("Error receiving event: {:?}", e);
                      break;
                  }
            };
          },
          // bind to the service
          // shutdown daemon
          _ = cancellation.cancelled() => {
              let receiver = mdns.unregister(&service_fullname).unwrap();
              while let Ok(status) = receiver.recv() {
                  match status {
                    UnregisterStatus::OK => {
                      println!("Service {service_fullname} unregistered");
                      return Ok(());
                    }
                    UnregisterStatus::NotFound => {
                      println!("Service {service_fullname} not found!");
                      return Ok(());
                    }
                  }
              }
          },

        }
    }

    Ok(())
}
