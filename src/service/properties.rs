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
    /// Name of the protocol expected by the address, e.g. `grpc`, `http`, `ws`, etc.
    pub protocol: String,
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
        protocol: String,
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
            // cant be busy at the start
            is_busy: false,
            mem,
            cpus,
            gpus,
            is_manager,
            hostname,
            instance,
            address,
            protocol,
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
            "none".to_string(),
        );

        println!("Service Properties: {:#?}", props.into_txt_properties());
    }
}
