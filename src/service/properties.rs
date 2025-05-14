use mdns_sd::{IntoTxtProperties, TxtProperties};
use std::collections::HashMap;

/// A collection of metrics about a service instance.
///
/// Implement [`IntoTxtProperties`] so that these properties can be used
/// in mDNS `TXT` records.
///
/// Also implements [`From<TxtProperties>`] so that these properties can be
/// repopulated from mDNS `TXT` records.
#[derive(Default, Clone, Debug)]
pub struct ServiceProperties {
    /// Amount of available memory in RAM, in bytes.
    ///
    /// On top of `free_memory`, this is the amount of memory that can be re-used as well.
    available_memory: u64,
    /// Amount of free memory in RAM, in bytes.
    ///
    /// In Windows / FreeBSD this is the same as `available_memory`.
    free_memory: u64,
    /// Total amount of memory in RAM, in bytes.
    total_memory: u64,
    /// Whether this service is a leader or not.
    ///
    /// We only expect there to be a single leader in the local network.
    is_leader: bool,
    /// Whether this service is currently doing a task or not.
    is_busy: bool,
}

impl ServiceProperties {
    /// Repopulates the properties with the given [`sysinfo::System`] instance.
    pub fn refresh_sysinfo(&mut self, sysinfo: &sysinfo::System) {
        self.available_memory = sysinfo.available_memory();
        self.free_memory = sysinfo.free_memory();
        self.total_memory = sysinfo.total_memory();
    }

    /// Sets `is_leader` to `true`.
    ///
    /// This should only be done when the service is attempted to be running
    /// as a leader / coordinator. It must be the case that all other workers
    /// within the local network are not leaders, and this is the only one.
    ///
    /// If there is a collision, the service will close itself in favor of the
    /// other existing leader.
    pub fn become_leader(&mut self) {
        self.is_leader = true;
    }
}

impl From<TxtProperties> for ServiceProperties {
    fn from(txt_properties: TxtProperties) -> Self {
        let mut props = Self::default();
        for (key, value) in txt_properties.into_property_map_str() {
            match key.as_str() {
                "mem_avail" => props.available_memory = value.parse().unwrap_or_default(),
                "mem_free" => props.free_memory = value.parse().unwrap_or_default(),
                "mem_total" => props.total_memory = value.parse().unwrap_or_default(),
                "is_leader" => props.is_leader = value.parse().unwrap_or_default(),
                "is_busy" => props.is_busy = value.parse().unwrap_or_default(),
                _ => {}
            }
        }
        props
    }
}

impl IntoTxtProperties for &ServiceProperties {
    fn into_txt_properties(self) -> TxtProperties {
        let props = HashMap::from_iter(
            [
                ("mem_avail", self.available_memory),
                ("mem_free", self.free_memory),
                ("mem_total", self.total_memory),
                ("is_leader", self.is_leader.into()),
                ("is_busy", self.is_busy.into()),
            ]
            .map(|(k, v)| (k.to_string(), v.to_string())),
        );

        // check lengths
        for (key, value) in props.iter() {
            if key.len() + value.len() > 255 {
                log::warn!("Property {} exceeds 255 bytes", key);
            }
        }

        props.into_txt_properties()
    }
}
