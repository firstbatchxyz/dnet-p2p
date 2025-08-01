/// Core service functionality.
mod core;
pub use core::DnetService;

/// Service properties, also used in mDNS as `TXT` records.
mod properties;
pub use properties::DnetServiceProperties;

/// mDNS specific functions.
mod mdns;
#[cfg(test)]
mod tests {
    use wgpu::Backends;
    use wgpu::Instance;

    #[test]
    fn test_gpu_detection() {
        let instance = Instance::new(&wgpu::InstanceDescriptor::default());
        for adapter in instance.enumerate_adapters(Backends::all()) {
            let info = adapter.get_info();
            println!("GPU Info: {:#?}", info);
        }
    }
}
