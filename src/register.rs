use mdns_sd::{DaemonEvent, IfKind, ServiceDaemon, ServiceInfo};
use std::{thread, time::Duration};

pub fn register_service(
    service_type: String,
    instance_name: String,
    hostname: String,
    will_unregister: bool,
    disable_ipv6: bool,
) {
    // Create a new mDNS daemon.
    let mdns = ServiceDaemon::new().expect("Could not create service daemon");

    if disable_ipv6 {
        mdns.disable_interface(IfKind::IPv6).unwrap();
    }

    // With `enable_addr_auto()`, we can give empty addrs and let the lib find them.
    // If the caller knows specific addrs to use, then assign the addrs here.
    let my_addrs = "";
    let service_hostname = format!("{}.local.", hostname);
    let port = 3456;

    // The key string in TXT properties is case insensitive. Only the first
    // (key, val) pair will take effect.
    let properties = [("PATH", "one"), ("Path", "two"), ("PaTh", "three")];

    // Register a service.
    let service_info = ServiceInfo::new(
        &service_type,
        &instance_name,
        &service_hostname,
        my_addrs,
        port,
        &properties[..],
    )
    .expect("valid service info")
    .enable_addr_auto();

    // Optionally, we can monitor the daemon events.
    let monitor = mdns.monitor().expect("Failed to monitor the daemon");
    let service_fullname = service_info.get_fullname().to_string();
    mdns.register(service_info)
        .expect("Failed to register mDNS service");

    println!("Registered service {}.{}", &instance_name, &service_type);

    if will_unregister {
        let wait_in_secs = 2;
        println!("Sleeping {} seconds before unregister", wait_in_secs);
        thread::sleep(Duration::from_secs(wait_in_secs));

        let receiver = mdns.unregister(&service_fullname).unwrap();
        while let Ok(event) = receiver.recv() {
            println!("unregister result: {:?}", &event);
        }
    } else {
        // Monitor the daemon events.
        while let Ok(event) = monitor.recv() {
            println!("Daemon event: {:?}", &event);
            if let DaemonEvent::Error(e) = event {
                println!("Failed: {}", e);
                break;
            }
        }
    }
}
