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
    pub mem_avail: u64,
    /// Amount of free memory in RAM, in bytes.
    ///
    /// In Windows / FreeBSD this is the same as `mem_avail`.
    pub mem_free: u64,
    /// Total amount of memory in RAM, in bytes.
    pub mem_total: u64,
    /// Number of CPUs available to this service.
    pub num_cpus: u32,
    /// Brand of the CPU.
    pub cpu_brand: String,
    /// Whether this service is a manager or not.
    ///
    /// We only expect there to be a single manager in the local network.
    pub is_manager: bool,
    /// Whether this service is currently doing a task or not.
    pub is_busy: bool,

    /// Connection information for the service.
    pub hostname: String,
    pub instance_name: String,
}

impl DnetServiceProperties {
    /// Creates a new instance of [`ServiceProperties`] with the given [`sysinfo::System`] instance.
    ///
    /// [`Self::refresh_sysinfo`] is called immediately to populate the memory properties.
    pub fn new(
        sysinfo: &sysinfo::System,
        is_manager: bool,
        hostname: String,
        instance_name: String,
    ) -> Self {
        let mut props = Self {
            is_manager,
            hostname,
            instance_name,
            // these will be refreshed just below
            mem_avail: 0,
            mem_free: 0,
            mem_total: 0,
            num_cpus: 0,
            cpu_brand: sysinfo
                .cpus()
                .first()
                .map_or_else(|| "Unknown".to_string(), |cpu| cpu.brand().to_string()),
            // cant be busy at the start
            is_busy: false,
        };

        props.refresh_sysinfo(sysinfo);

        props
    }

    /// Repopulates the properties with the given [`sysinfo::System`] instance.
    pub fn refresh_sysinfo(&mut self, sysinfo: &sysinfo::System) {
        self.mem_avail = sysinfo.available_memory();
        self.mem_free = sysinfo.free_memory();
        self.mem_total = sysinfo.total_memory();
        self.num_cpus = sysinfo.cpus().len() as u32;
    }
}

impl From<&TxtProperties> for DnetServiceProperties {
    fn from(props: &TxtProperties) -> Self {
        Self {
            mem_avail: props
                .get_property_val_str("mem_avail")
                .and_then(|s| s.parse().ok())
                .unwrap_or_default(),
            mem_free: props
                .get_property_val_str("mem_free")
                .and_then(|s| s.parse().ok())
                .unwrap_or_default(),
            mem_total: props
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
            hostname: props
                .get_property_val_str("hostname")
                .unwrap_or_default()
                .to_string(),
            instance_name: props
                .get_property_val_str("instance_name")
                .unwrap_or_default()
                .to_string(),
            num_cpus: props
                .get_property_val_str("num_cpus")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
            cpu_brand: props
                .get_property_val_str("cpu_brand")
                .map(|s| s.to_string())
                .unwrap_or_else(|| "Unknown".to_string()),
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
                ("mem_avail", self.mem_avail.to_string()),
                ("mem_free", self.mem_free.to_string()),
                ("mem_total", self.mem_total.to_string()),
                ("is_manager", self.is_manager.to_string()),
                ("is_busy", self.is_busy.to_string()),
                ("hostname", self.hostname.to_string()),
                ("instance_name", self.instance_name.to_string()),
                ("num_cpus", self.num_cpus.to_string()),
                ("cpu_brand", self.cpu_brand.to_string()),
            ]
            // map keys to strings
            .map(|(k, v)| (k.to_string(), v)),
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
