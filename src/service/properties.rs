use mdns_sd::{IntoTxtProperties, TxtProperties};
use serde::{Deserialize, Serialize};
use serde_txtrecord::{from_txt_records, to_txt_records};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnetServiceCPUProperties {
    /// Brand of the CPU.
    pub brand: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnetServiceGPUProperties {
    /// Name of the GPU.
    pub name: String,
    /// Type of the GPU, e.g. "Integrated" or "Dedicated".
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnetServiceMemoryProperties {
    /// Amount of available memory in RAM, in bytes.
    ///
    /// On top of `free_memory`, this is the amount of memory that can be re-used as well.
    pub avail: u64,
    /// Amount of free memory in RAM, in bytes.
    ///
    /// In Windows / FreeBSD this is the same as `avail`.
    pub free: u64,
    /// Total amount of memory in RAM, in bytes.
    pub total: u64,
}

/// A collection of metrics about a service instance.
///
/// - Can be converted to/from JSON via [`serde_json`].
/// - Can be converted to/from TXT records via [`serde_txtrecord`].
///
/// NOTE: We are not using `repr(C)` in particular, because we are interested in a hashmap
/// where this struct is the value, and the keys are strings (peer ids). So the natural
/// thing to do is to serialize this to a JSON string to pass via FFI.
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
    pub server_port: u16,
    /// Shard port for this device, can be using a custom socket protocol or gRPC.
    pub shard_port: u16,
}

impl DnetServiceProperties {
    /// Creates a new instance of [`ServiceProperties`] with the given [`sysinfo::System`] instance.
    ///
    /// [`Self::refresh_sysinfo`] is called immediately to populate the memory properties.
    pub fn new(
        is_manager: bool,
        instance: String,
        host: String,
        server_port: u16,
        shard_port: u16,
    ) -> Self {
        Self {
            is_busy: false,
            is_manager,
            instance,
            host,
            server_port,
            shard_port,
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
    fn test_properties() {
        let props = DnetServiceProperties::new(
            true,
            "localhost".to_string(),
            "127.0.0.1".to_string(),
            8080,
            8081,
        );

        println!("Service Properties: {:#?}", props.into_txt_properties());
    }
}
