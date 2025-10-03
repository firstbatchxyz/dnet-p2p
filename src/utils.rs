/// Returns the private IP address of the local machine.
///
/// This is so that other devices in the same local network can reach this service.
///
/// It specifically looks for private IPv4 addresses in `en` interfaces, picking the first one.
pub fn get_local_network_ip() -> Option<(String, std::net::IpAddr)> {
    let mut en_ifs = local_ip_address::list_afinet_netifas()
        .unwrap()
        .into_iter()
        .filter(|(name, addr)| name.starts_with("en") && addr.is_ipv4())
        .collect::<Vec<_>>();
    en_ifs.sort();

    en_ifs.first().cloned()
}

/// Returns the IP from the bridge interface `bridge0`.
///
/// - This is active when using Thunderbolt networking on macOS.
/// - FOr other OS'es
pub fn get_bridge_ip() -> Option<(String, std::net::IpAddr)> {
    local_ip_address::list_afinet_netifas()
        .unwrap()
        .into_iter()
        .find(|(name, addr)| name == "bridge0" && addr.is_ipv4())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ips() {
        let local_ip = get_local_network_ip();
        println!("Local network IP: {:?}", local_ip);

        let bridge_ip = get_bridge_ip();
        println!("Bridge IP: {:?}", bridge_ip);
    }
}
