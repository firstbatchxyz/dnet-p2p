use mdns_sd::{IntoTxtProperties, TxtProperties};
use serde::{Deserialize, Serialize};
use serde_txtrecord::{from_txt_records, to_txt_records};
use std::collections::HashMap;

use crate::service::ThunderboltData;

/// A collection of metrics about a service instance.
///
/// - Can be converted to/from JSON via [`serde_json`].
/// - Can be converted to/from TXT records via [`serde_txtrecord`].
///
/// NOTE: We are not using [`repr(C)`](https://doc.rust-lang.org/nomicon/other-reprs.html#reprc) in particular,
/// because we are interested in a hashmap where this struct is the value, and the keys are strings (peer ids).
/// So the natural thing to do is to serialize this to a JSON string to pass via FFI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnetServiceProperties {
    /// Whether this service is a manager or not.
    ///
    /// We only expect there to be a single manager in the local network.
    pub is_manager: bool,
    /// Whether this service is currently doing a task or not.
    ///
    /// This defaults to `false` on creation, and should be updated by the service itself.
    pub is_busy: bool,
    /// The instance name of this service.
    ///
    /// If multiple instances exist, the new one will have a number appended to it,
    /// e.g. `foobar`, `foobar (2)`, etc.
    pub instance: String,
    /// Host address of the service, e.g. "127.0.0.1".
    pub host: String,
    /// HTTP server port for this device.
    ///
    /// Can be used as `{host}:{server_port}` to reach the HTTP API.
    pub server_port: u16,
    /// Shard port for this device, can be using a custom socket protocol or gRPC.
    ///
    /// Can be used as `{host}:{shard_port}` to reach the shard service.
    pub shard_port: u16,
    /// Local IP address of the service, e.g. "192.168.1.2".
    ///
    /// This can be used to reach the service from other devices in the same network.
    pub local_ip: String,
    /// Thunderbolt-related connection information, if any.
    pub thunderbolt: Option<ThunderboltData>,
}

impl DnetServiceProperties {
    /// Creates a new instance of [`ServiceProperties`] with the given [`sysinfo::System`] instance.
    pub fn new(
        is_manager: bool,
        instance: String,
        host: String,
        server_port: u16,
        shard_port: u16,
        local_ip: String,
    ) -> Self {
        let mut service = Self {
            is_busy: false,
            is_manager,
            instance,
            host,
            server_port,
            shard_port,
            local_ip,
            thunderbolt: None,
        };

        // post-creation checks
        service.detect_thunderbolt();

        service
    }

    /// Detects if there is a Thunderbolt connection and updates the properties accordingly.
    ///
    /// Only works on macOS.
    pub fn detect_thunderbolt(&mut self) {
        #[cfg(target_os = "macos")]
        {
            match ThunderboltData::new_from_profile() {
                Ok(tb_info) => {
                    log::info!("Detected Thunderbolt connection: {:#?}", tb_info);
                    self.thunderbolt = Some(tb_info);
                }
                Err(e) => {
                    log::warn!("Could not detect Thunderbolt connection: {}", e);
                    self.thunderbolt = None;
                }
            }
        }
    }
}

impl TryFrom<&TxtProperties> for DnetServiceProperties {
    type Error = serde_txtrecord::DeserializeError;

    /// Converts the `TxtProperties` into a `DnetServiceProperties` instance.
    /// This will fail if the properties cannot be parsed correctly.
    fn try_from(props: &TxtProperties) -> Result<Self, Self::Error> {
        let keys_values: Vec<(String, String)> = props
            .iter()
            .map(|e| (e.key().to_string(), e.val_str().to_string()))
            .collect();

        from_txt_records(keys_values)
    }
}

impl IntoTxtProperties for &DnetServiceProperties {
    /// Converts the service properties into a `TxtProperties` instance.
    fn into_txt_properties(self) -> TxtProperties {
        let txt_records =
            to_txt_records(self).expect("could not serialize service properties to TXT records");

        HashMap::from_iter(txt_records.into_iter()).into_txt_properties()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_txt_properties() {
        let props = DnetServiceProperties::new(
            true,
            "localhost".to_string(),
            "127.0.0.1".to_string(),
            8080,
            8081,
            "192.168.1.2".to_string(),
        );

        println!("Service Properties: {:#?}", props.into_txt_properties());
    }
}
