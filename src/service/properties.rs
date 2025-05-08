use std::collections::HashMap;

use mdns_sd::{IntoTxtProperties, TxtProperties};

pub struct Properties {
    sysinfo: sysinfo::System,
}

impl Properties {
    // TODO: use sysinfo
    pub fn new() -> Self {
        Self {
            sysinfo: sysinfo::System::new_all(),
        }
    }
}

impl IntoTxtProperties for &Properties {
    fn into_txt_properties(self) -> TxtProperties {
        let mut props = HashMap::new();
        props.insert(
            "mem_avail".to_string(),
            self.sysinfo.available_memory().to_string(),
        );
        props.insert(
            "mem_free".to_string(),
            self.sysinfo.free_memory().to_string(),
        );
        props.insert(
            "mem_total".to_string(),
            self.sysinfo.total_memory().to_string(),
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
