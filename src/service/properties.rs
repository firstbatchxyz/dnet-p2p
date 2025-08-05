use mdns_sd::{IntoTxtProperties, TxtProperties};
use serde::{Deserialize, Serialize};
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
/// Implement [`IntoTxtProperties`] so that these properties can be used
/// in mDNS `TXT` records.
///
/// Also implements [`From<TxtProperties>`] so that these properties can be
/// repopulated from mDNS `TXT` records.
///
/// We are not using `repr(C)` in particular, because we are interested in a hashmap
/// where this struct is the value, and the keys are strings (peer ids). So the natural
/// thing to do is to serialize this to a JSON string to pass via FFI.
///
/// TODO: Boolean fields are represented as `1` for `true` and `0` for `false` in the `TxtProperties`.
/// However, as per RFC 6763, we could maybe simply omit them for `false` and put them without a value for `true`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnetServiceProperties {
    pub mem: DnetServiceMemoryProperties,
    pub cpus: Vec<DnetServiceCPUProperties>,
    pub gpus: Vec<DnetServiceGPUProperties>,

    /// Whether this service is a manager or not.
    ///
    /// We only expect there to be a single manager in the local network.
    pub is_manager: bool,
    /// Whether this service is currently doing a task or not.
    pub is_busy: bool,

    /// Hostname of the machine.
    pub hostname: String,
    /// Instance name of the service, e.g. "worker-1".
    pub instance: String,
    /// Address of the service (e.g. `{host}:{port}`), can be used to connect to it.
    pub address: String,
}

impl DnetServiceProperties {
    /// Creates a new instance of [`ServiceProperties`] with the given [`sysinfo::System`] instance.
    ///
    /// [`Self::refresh_sysinfo`] is called immediately to populate the memory properties.
    pub fn new(
        sysinfo: &sysinfo::System,
        gpuinfo: &wgpu::Instance,
        is_manager: bool,
        hostname: String,
        instance: String,
        address: String,
    ) -> Self {
        let gpus = gpuinfo
            .enumerate_adapters(wgpu::Backends::all())
            .into_iter()
            .map(|adapter| DnetServiceGPUProperties {
                name: adapter.get_info().name.to_string(),
                kind: match adapter.get_info().device_type {
                    wgpu::DeviceType::Other => "Other".to_string(),
                    wgpu::DeviceType::IntegratedGpu => "Integrated".to_string(),
                    wgpu::DeviceType::DiscreteGpu => "Discrete".to_string(),
                    wgpu::DeviceType::VirtualGpu => "Virtual".to_string(),
                    wgpu::DeviceType::Cpu => "CPU".to_string(),
                },
            })
            .collect();

        let cpus = sysinfo
            .cpus()
            .iter()
            .map(|cpu| DnetServiceCPUProperties {
                brand: cpu.brand().to_string(),
            })
            .collect();

        let mem = DnetServiceMemoryProperties {
            avail: sysinfo.available_memory(),
            free: sysinfo.free_memory(),
            total: sysinfo.total_memory(),
        };
        let mut props = Self {
            mem,
            cpus,
            gpus,
            //------------ ANY ------------//
            // cant be busy at the start
            is_busy: false,
            is_manager,
            hostname,
            instance,
            address,
        };

        props.refresh_sysinfo(sysinfo);

        props
    }

    /// Repopulates the properties with the given [`sysinfo::System`] instance.
    pub fn refresh_sysinfo(&mut self, sysinfo: &sysinfo::System) {
        self.mem.avail = sysinfo.available_memory();
        self.mem.free = sysinfo.free_memory();
        self.mem.total = sysinfo.total_memory();
    }
}

impl From<&TxtProperties> for DnetServiceProperties {
    fn from(props: &TxtProperties) -> Self {
        // memory stuff, all are `u64`
        let [mem_avail, mem_free, mem_total] = ["mem_avail", "mem_free", "mem_total"].map(|key| {
            props
                .get_property_val_str(key)
                .and_then(|s| s.parse().ok())
                .unwrap_or_default()
        });

        // CPU properties
        let num_cpus = props
            .get_property_val_str("num_cpus")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let mut cpus = Vec::with_capacity(num_cpus as usize);
        for i in 0..num_cpus {
            let prefix = format!("cpu_{i}");
            cpus.push(DnetServiceCPUProperties {
                brand: props
                    .get_property_val_str(&format!("{}.brand", prefix))
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        // GPU properties
        let num_gpus = props
            .get_property_val_str("num_gpus")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let mut gpus = Vec::with_capacity(num_gpus as usize);
        for i in 0..num_gpus {
            let prefix = format!("gpu_{i}");
            gpus.push(DnetServiceGPUProperties {
                name: props
                    .get_property_val_str(&format!("{}.name", prefix))
                    .unwrap_or_default()
                    .to_string(),
                kind: props
                    .get_property_val_str(&format!("{}.kind", prefix))
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        // strings
        let [hostname, instance, address] = ["hostname", "instance", "address"].map(|key| {
            props
                .get_property_val_str(key)
                .unwrap_or_default()
                .to_string()
        });

        // booleans
        let [is_manager, is_busy] =
            ["is_manager", "is_busy"].map(|key| props.get_property_val_str(key).is_some());

        Self {
            mem: DnetServiceMemoryProperties {
                avail: mem_avail,
                free: mem_free,
                total: mem_total,
            },
            hostname,
            instance,
            address,
            cpus,
            gpus,
            is_manager,
            is_busy,
        }
    }
}

impl IntoTxtProperties for &DnetServiceProperties {
    /// Converts the service properties into a `TxtProperties` instance.
    ///
    /// Each key-value pair is converted to a string representation as `{key}={value}`
    /// and must not exceed 255 bytes in total length, as per [RFC 6763 Sec. 6](https://www.rfc-editor.org/rfc/rfc6763.html#section-6).
    fn into_txt_properties(self) -> TxtProperties {
        let mut props = HashMap::from_iter(
            [
                ("hostname", self.hostname.to_string()),
                ("instance", self.instance.to_string()),
                ("address", self.address.to_string()),
                ("mem_avail", self.mem.avail.to_string()),
                ("mem_free", self.mem.free.to_string()),
                ("mem_total", self.mem.total.to_string()),
            ]
            // map keys to strings
            .map(|(k, v)| (k.to_string(), v)),
        );

        // bools
        if self.is_manager {
            props.insert("is_manager".to_string(), String::default());
        }
        if self.is_busy {
            props.insert("is_busy".to_string(), String::default());
        }

        // add CPU and GPU brands
        props.insert("num_cpus".to_string(), self.cpus.len().to_string());
        for (i, cpu) in self.cpus.iter().enumerate() {
            props.insert(format!("cpu_{i}.brand"), cpu.brand.to_string());
        }
        props.insert("num_gpus".to_string(), self.gpus.len().to_string());
        for (i, gpu) in self.gpus.iter().enumerate() {
            props.insert(format!("gpu_{i}.name"), gpu.name.to_string());
            props.insert(format!("gpu_{i}.kind"), gpu.kind.to_string());
        }

        // check lengths, must not exceed 255 bytes
        for (key, value) in props.iter() {
            if key.len() + value.len() > 255 {
                log::warn!("Property {key}={value} exceeds 255 bytes");
            }
        }
        props.into_txt_properties()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sysinfo;
    use wgpu::{Backends, Instance};

    #[test]
    fn test_gpu_detection() {
        let instance = Instance::new(&wgpu::InstanceDescriptor::default());
        for adapter in instance.enumerate_adapters(Backends::all()) {
            let info = adapter.get_info();
            println!("GPU Info: {:#?}", info);
        }
    }

    #[test]
    fn test_cpu_detection() {
        let mut sys = sysinfo::System::new_all();
        sys.refresh_all();
        for cpu in sys.cpus() {
            println!(
                "CPU: {} ({} - {} MHz) at {}% usage",
                cpu.name(),
                cpu.brand(),
                cpu.frequency(),
                cpu.cpu_usage()
            );
        }
    }

    #[test]
    fn test_properties() {
        let sysinfo = sysinfo::System::new_all();
        let gpuinfo = Instance::new(&wgpu::InstanceDescriptor::default());
        let props = DnetServiceProperties::new(
            &sysinfo,
            &gpuinfo,
            true,
            "localhost".to_string(),
            "test_instance".to_string(),
            "127.0.0.1".to_string(),
        );

        println!("Service Properties: {:#?}", props.into_txt_properties());
    }
}
