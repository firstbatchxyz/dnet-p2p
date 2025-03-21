use mdns_sd::{ServiceDaemon, ServiceEvent};

/// Query the mDNS to discover services, with respect to `service_type`.
///
/// Service type must end with `._udp.local.` or `._tcp.local.`.
///
/// You can also do a meta-query per RFC 6763 to find which services are available by providing
/// `_services._dns-sd._udp` as the service type.
pub fn query_services(mut service_type: String) {
    // Create a daemon
    let mdns = ServiceDaemon::new().expect("Failed to create daemon");

    // append `.local.` f
    if !service_type.ends_with(".local.") {
        service_type.push_str(".local.");
    }
    let receiver = mdns.browse(&service_type).expect("Failed to browse");

    let now = std::time::Instant::now();
    while let Ok(event) = receiver.recv() {
        match event {
            ServiceEvent::ServiceResolved(info) => {
                println!(
                    "At {:?}: Resolved a new service: {}\n host: {}\n port: {}",
                    now.elapsed(),
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
            }
            other_event => {
                println!("At {:?}: {:?}", now.elapsed(), &other_event);
            }
        }
    }
}
