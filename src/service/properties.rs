use mdns_sd::{IntoTxtProperties, TxtProperties};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A collection of metrics about a service instance.
///
/// Implement [`IntoTxtProperties`] so that these properties can be used
/// in mDNS `TXT` records.
///
/// Also implements [`From<TxtProperties>`] so that these properties can be
/// repopulated from mDNS `TXT` records.
///
/// We are not using `repr(C)` in particular, because we are interested in a hashmap
/// where this struct is the value, and the keys are strings (peer ids). So the natural
/// thing to do is to serialize this to a JSON string to pass via FFI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(C)]
pub struct DnetServiceProperties {
    /// Amount of available memory in RAM, in bytes.
    ///
    /// On top of `free_memory`, this is the amount of memory that can be re-used as well.
    pub available_memory: u64,
    /// Amount of free memory in RAM, in bytes.
    ///
    /// In Windows / FreeBSD this is the same as `available_memory`.
    pub free_memory: u64,
    /// Total amount of memory in RAM, in bytes.
    pub total_memory: u64,
    /// Whether this service is a manager or not.
    ///
    /// We only expect there to be a single manager in the local network.
    pub is_manager: bool,
    /// Whether this service is currently doing a task or not.
    pub is_busy: bool,
}

impl DnetServiceProperties {
    /// Creates a new instance of [`ServiceProperties`] with the given [`sysinfo::System`] instance.
    ///
    /// [`Self::refresh_sysinfo`] is called immediately to populate the memory properties.
    pub fn new(sysinfo: &sysinfo::System, is_manager: bool) -> Self {
        let mut props = Self {
            // these will be refreshed just below
            available_memory: 0,
            free_memory: 0,
            total_memory: 0,
            // cant be busy at the start
            is_busy: false,
            is_manager,
        };

        props.refresh_sysinfo(sysinfo);

        props
    }

    /// Repopulates the properties with the given [`sysinfo::System`] instance.
    pub fn refresh_sysinfo(&mut self, sysinfo: &sysinfo::System) {
        self.available_memory = sysinfo.available_memory();
        self.free_memory = sysinfo.free_memory();
        self.total_memory = sysinfo.total_memory();
    }
}

impl From<&TxtProperties> for DnetServiceProperties {
    fn from(props: &TxtProperties) -> Self {
        Self {
            available_memory: props
                .get_property_val_str("mem_avail")
                .and_then(|s| s.parse().ok())
                .unwrap_or_default(),
            free_memory: props
                .get_property_val_str("mem_free")
                .and_then(|s| s.parse().ok())
                .unwrap_or_default(),
            total_memory: props
                .get_property_val_str("mem_total")
                .and_then(|s| s.parse().ok())
                .unwrap_or_default(),
            is_manager: props
                .get_property_val_str("is_manager")
                .map(|s| s == "1")
                .unwrap_or(false),
            is_busy: props
                .get_property_val_str("is_busy")
                .map(|s| s == "1")
                .unwrap_or(false),
        }
    }
}

impl IntoTxtProperties for &DnetServiceProperties {
    /// Converts the service properties into a `TxtProperties` instance.
    ///
    /// Each key-value pair is converted to a string representation as `{key}={value}`
    /// and must not exceed 255 bytes in total length, as per [RFC 6763 Sec. 6](https://www.rfc-editor.org/rfc/rfc6763.html#section-6).
    fn into_txt_properties(self) -> TxtProperties {
        let props = HashMap::from_iter(
            [
                ("mem_avail", self.available_memory),
                ("mem_free", self.free_memory),
                ("mem_total", self.total_memory),
                ("is_manager", self.is_manager.into()),
                ("is_busy", self.is_busy.into()),
            ]
            .map(|(k, v)| (k.to_string(), v.to_string())),
        );

        // check lengths, must not exceed 255 bytes
        for (key, value) in props.iter() {
            if key.len() + value.len() > 255 {
                log::warn!("Property {key}={value} exceeds 255 bytes");
            }
        }

        props.into_txt_properties()
    }
}
