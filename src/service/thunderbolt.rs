use serde::{Deserialize, Serialize};
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThunderboltInstance {
    /// Domain UUID of the device, from `domain_uuid_key`.
    pub uuid: String,
    /// Name fo the connection, e.g. "thunderboltusb4_bus_2" or "Macbook Air", from `_name`.
    pub name: String,
    /// Human-readable name of the device, e.g. "Mac15,12", from `device_name_key`.
    pub device: String,
}

impl ThunderboltInstance {
    pub fn from_value(value: &serde_json::Value) -> Option<Self> {
        Some(Self {
            uuid: value["domain_uuid_key"].as_str()?.to_string(),
            name: value["_name"].as_str()?.to_string(),
            device: value["device_name_key"].as_str()?.to_string(),
        })
    }
}

/// Thunderbolt-related connection information.
///
/// The information here can be obtained via:
///
/// ```sh
/// system_profiler SPNetworkDataType SPThunderboltDataType -json
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThunderboltData {
    /// Thunderbolt ip addresses.
    ///
    /// This is the IP address of the Thunderbolt interface.
    /// It can be defined manually by the user, or automatically via DHCP.
    ///
    /// This can be obtained via `SPNetworkDataType` profile.
    pub ip_addrs: Vec<String>,
    /// Domain UUIDs of Thunderbolt ports, and their connections.
    ///
    /// Each physical port has a domain UUID.
    /// When another device connects to this device via Thunderbolt,
    /// it will see that exact domain UUID within it's connected `_items` list.
    /// This way, we can identify which device is connected via Thunderbolt.
    ///
    /// This can be obtained via `SPThunderboltDataType` profile.
    pub instances: Vec<(ThunderboltInstance, Vec<ThunderboltInstance>)>,
}

// FIXME: how to learn which domain_uuid belongs to which host?

impl ThunderboltData {
    pub fn new_from_slice(data: &[u8]) -> Option<Self> {
        let json: serde_json::Value = serde_json::from_slice(data).expect("failed to parse JSON");

        // extract thunder ip addresses from network information
        let mut ip_addrs = Vec::new();
        if let Some(network_array) = json["SPNetworkDataType"].as_array() {
            for item in network_array.iter() {
                if let Some(name) = item["_name"].as_str() {
                    // FIXME: we are specifically checking a static name here, but
                    // a power-user might have changed the value of this one
                    if name == "Thunderbolt Bridge" {
                        ip_addrs =
                            serde_json::from_value(item["ip_address"].clone()).unwrap_or_default();
                        break;
                    }
                }
            }
        }

        // extract thunderbolt instances and connections
        let mut instances = Vec::new();
        if let Some(thunderbolt_array) = json["SPThunderboltDataType"].as_array() {
            for item in thunderbolt_array.iter() {
                // extract info about myself

                // extract info about
                // Extract the `domain_uuid_key` for this port as the key
                if let Some(myself) = ThunderboltInstance::from_value(item) {
                    let mut connected_devices = Vec::new();

                    // Check if this port has connected devices in `_items`
                    if let Some(items_array) = item["_items"].as_array() {
                        for connected_item in items_array.iter() {
                            if let Some(instance) = ThunderboltInstance::from_value(connected_item)
                            {
                                connected_devices.push(instance);
                            }
                        }
                    }

                    // Insert the port with its connected devices (empty if no connections)
                    instances.push((myself, connected_devices));
                }
            }
        }

        Some(Self {
            ip_addrs,
            instances,
        })
    }

    pub async fn new_from_profile() -> Option<Self> {
        // execute `system_profiler SPNetworkDataType SPThunderboltDataType -json` and read output
        let output = Command::new("system_profiler")
            .arg("SPNetworkDataType") // for ips
            .arg("SPThunderboltDataType") // for domain_uuids and connections
            .arg("-json")
            .output()
            .await
            .expect("failed to execute process"); // FIXME: !!!

        if !output.status.success() {
            log::error!(
                "system_profiler command failed with status: {}",
                output.status
            );
            None
        } else {
            // println!(
            //     "Thunderbolt profile output:\n\n{}",
            //     String::from_utf8_lossy(&output.stdout)
            // );
            Self::new_from_slice(&output.stdout)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_thunderbolt_info() {
        let tb_info = ThunderboltData::new_from_profile().await;
        println!("{:#?}", tb_info);
        assert!(tb_info.is_some());
    }

    #[test]
    fn test_ports_parsing_with_sample_data() {
        let sample_json = r#"{
    "SPThunderboltDataType": [
    {
      "_name": "thunderboltusb4_bus_2",
      "_items": [
      {
        "_name": "Device 1",
        "device_id_key": "0xA",
        "device_name_key": "Device15,12",
        "domain_uuid_key": "12345678-1234-5678-9876-543210987654",
        "services_title": [
        {
          "_name": "service_ip",
          "protocol_id_key": 1,
          "protocol_revision_key": 1,
          "protocol_version_key": 1,
          "service_uuid_key": "11111111-2222-3333-4444-555555555555"
        }
        ],
        "vendor_id_key": "0xA27",
        "vendor_name_key": "Company Inc."
      }
      ],

      "device_name_key": "Host Device",
      "domain_uuid_key": "87654321-4321-8765-1234-567890123456",
      "receptacle_1_tag": {
      "current_speed_key": "40 Gb/s",
      "link_status_key": "0x2",
      "receptacle_id_key": "3",
      "receptacle_status_key": "receptacle_connected"
      },
      "route_string_key": "0",
      "switch_uid_key": "0xABCDEF0123456789",
      "vendor_name_key": "Company Inc."
    },
    {
      "_name": "thunderboltusb4_bus_1",
      "_items": [
      {
        "_name": "Device 2",
        "device_id_key": "0xA",
        "device_name_key": "Device18,2",
        "domain_uuid_key": "98765432-5678-9012-3456-789012345678",
        "services_title": [
        {
          "_name": "service_ip",
          "protocol_id_key": 1,
          "protocol_revision_key": 1,
          "protocol_version_key": 1,
          "service_uuid_key": "66666666-7777-8888-9999-AAAAAAAAAAAA"
        }
        ],
        "vendor_id_key": "0xA27",
        "vendor_name_key": "Company Inc."
      }
      ],
      "device_name_key": "Host Device",
      "domain_uuid_key": "FEDCBA98-7654-3210-FEDC-BA9876543210",
      "receptacle_1_tag": {
      "current_speed_key": "40 Gb/s",
      "link_status_key": "0x2",
      "receptacle_id_key": "2",
      "receptacle_status_key": "receptacle_connected"
      },
      "route_string_key": "0",
      "switch_uid_key": "0x1234567890ABCDEF",
      "vendor_name_key": "Company Inc."
    },
    {
      "_items": [
      {
        "_name": "Audio Device",
        "device_id_key": "0xB",
        "device_name_key": "Audio Device",
        "device_revision_key": "0x1",
        "mode_key": "thunderbolt_three",
        "receptacle_upstream_ambiguous_tag": {
        "current_speed_key": "40 Gb/s",
        "lc_version_key": "0.31.0",
        "link_status_key": "0x2",
        "receptacle_status_key": "receptacle_connected"
        },
        "route_string_key": "1",
        "switch_uid_key": "0xAABBCCDDEEFF0011",
        "switch_version_key": "20.1",
        "vendor_id_key": "0x1176",
        "vendor_name_key": "Audio Company, Inc."
      }
      ],
      "_name": "thunderboltusb4_bus_0",
      "device_name_key": "Host Device",
      "domain_uuid_key": "ABCDEF01-2345-6789-ABCD-EF0123456789",
      "receptacle_1_tag": {
      "current_speed_key": "40 Gb/s",
      "link_status_key": "0x2",
      "receptacle_id_key": "1",
      "receptacle_status_key": "receptacle_connected"
      },
      "route_string_key": "0",
      "switch_uid_key": "0x9876543210FEDCBA",
      "vendor_name_key": "Company Inc."
    }
    ],
    "SPNetworkDataType": [
    {
      "_name": "Ethernet",
      "Ethernet": {
      "MAC Address": "aa:bb:cc:dd:ee:ff",
      "MediaOptions": [],
      "MediaSubType": "none"
      },
      "hardware": "Ethernet",
      "interface": "en0",
      "IPv4": {
      "ConfigMethod": "DHCP"
      },
      "IPv6": {
      "ConfigMethod": "Automatic"
      },
      "Proxies": {
      "ExceptionsList": ["*.local", "169.254/16"],
      "FTPPassive": "yes"
      },
      "spnetwork_service_order": 0,
      "type": "Ethernet"
    },
    {
      "_name": "Ethernet Adapter (en5)",
      "Ethernet": {
      "MAC Address": "11:22:33:44:55:66",
      "MediaOptions": [],
      "MediaSubType": "none"
      },
      "hardware": "Ethernet",
      "interface": "en5",
      "IPv4": {
      "ConfigMethod": "DHCP"
      },
      "IPv6": {
      "ConfigMethod": "Automatic"
      },
      "Proxies": {
      "ExceptionsList": ["*.local", "169.254/16"],
      "FTPPassive": "yes"
      },
      "spnetwork_service_order": 1,
      "type": "Ethernet"
    },
    {
      "_name": "Ethernet Adapter (en6)",
      "Ethernet": {
      "MAC Address": "77:88:99:aa:bb:cc",
      "MediaOptions": [],
      "MediaSubType": "none"
      },
      "hardware": "Ethernet",
      "interface": "en6",
      "IPv4": {
      "ConfigMethod": "DHCP"
      },
      "IPv6": {
      "ConfigMethod": "Automatic"
      },
      "Proxies": {
      "ExceptionsList": ["*.local", "169.254/16"],
      "FTPPassive": "yes"
      },
      "spnetwork_service_order": 2,
      "type": "Ethernet"
    },
    {
      "_name": "Ethernet Adapter (en7)",
      "Ethernet": {
      "MAC Address": "dd:ee:ff:00:11:22",
      "MediaOptions": [],
      "MediaSubType": "none"
      },
      "hardware": "Ethernet",
      "interface": "en7",
      "IPv4": {
      "ConfigMethod": "DHCP"
      },
      "IPv6": {
      "ConfigMethod": "Automatic"
      },
      "Proxies": {
      "ExceptionsList": ["*.local", "169.254/16"],
      "FTPPassive": "yes"
      },
      "spnetwork_service_order": 3,
      "type": "Ethernet"
    },
    {
      "_name": "Thunderbolt Bridge",
      "Ethernet": {
      "MediaOptions": [],
      "MediaSubType": "autoselect"
      },
      "hardware": "Ethernet",
      "interface": "bridge0",
      "ip_address": ["169.254.100.50"],
      "IPv4": {
      "AdditionalRoutes": [
        {
        "DestinationAddress": "169.254.100.50",
        "SubnetMask": "255.255.255.255"
        }
      ],
      "Addresses": ["169.254.100.50"],
      "ConfigMethod": "DHCP",
      "ConfirmedInterfaceName": "bridge0",
      "InterfaceName": "bridge0",
      "SubnetMasks": ["255.255.0.0"]
      },
      "IPv6": {
      "ConfigMethod": "Automatic"
      },
      "Proxies": {
      "ExceptionsList": ["*.local", "169.254/16"],
      "FTPPassive": "yes"
      },
      "spnetwork_service_order": 4,
      "type": "Ethernet"
    },
    {
      "_name": "Wi-Fi",
      "dhcp": {
      "dhcp_domain_name_servers": "192.168.1.1",
      "dhcp_lease_duration": 0,
      "dhcp_message_type": "0x05",
      "dhcp_routers": "192.168.1.1",
      "dhcp_server_identifier": "192.168.1.1",
      "dhcp_subnet_mask": "255.255.255.0"
      },
      "DNS": {
      "ServerAddresses": ["192.168.1.1"]
      },
      "Ethernet": {
      "MAC Address": "33:44:55:66:77:88",
      "MediaOptions": [],
      "MediaSubType": "autoselect"
      },
      "hardware": "AirPort",
      "interface": "en1",
      "ip_address": ["192.168.1.100"],
      "IPv4": {
      "AdditionalRoutes": [
        {
        "DestinationAddress": "192.168.1.100",
        "SubnetMask": "255.255.255.255"
        },
        {
        "DestinationAddress": "169.254.0.0",
        "SubnetMask": "255.255.0.0"
        }
      ],
      "Addresses": ["192.168.1.100"],
      "ARPResolvedHardwareAddress": "aa:bb:cc:dd:ee:ff",
      "ARPResolvedIPAddress": "192.168.1.1",
      "ConfigMethod": "DHCP",
      "ConfirmedInterfaceName": "en1",
      "InterfaceName": "en1",
      "NetworkSignature": "IPv4.Router=192.168.1.1;IPv4.RouterHardwareAddress=aa:bb:cc:dd:ee:ff",
      "Router": "192.168.1.1",
      "SubnetMasks": ["255.255.255.0"]
      },
      "IPv6": {
      "Addresses": ["2001:db8::1234:5678:9abc:def0"],
      "ConfigMethod": "Automatic",
      "ConfirmedInterfaceName": "en1",
      "InterfaceName": "en1",
      "PrefixLength": [64]
      },
      "Proxies": {
      "ExceptionsList": ["*.local", "169.254/16"],
      "FTPPassive": "yes"
      },
      "spnetwork_service_order": 5,
      "type": "AirPort"
    }
    ]
  }
  "#;

        let tb_info = ThunderboltData::new_from_slice(sample_json.as_bytes()).unwrap();
        println!("{:#?}", tb_info);

        // print JSON
        println!("{}", serde_json::to_string_pretty(&tb_info).unwrap());

        // print TXT record
        println!("{:#?}", serde_txtrecord::to_txt_records(&tb_info).unwrap());
    }
}
