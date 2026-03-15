use std::ffi::CStr;
use std::sync::Mutex;

use ash::vk;
use gpu_allocator::vulkan::{Allocator, AllocatorCreateDesc};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GpuError {
    #[error("vulkan error: {0}")]
    Vulkan(#[from] vk::Result),
    #[error("no suitable GPU found")]
    NoDevice,
    #[error("shader compilation failed: {0}")]
    ShaderCompilation(String),
    #[error("buffer not mapped for access")]
    BufferNotMapped,
    #[error("frame source error: {0}")]
    FrameSource(String),
    #[error("allocator error: {0}")]
    Allocator(String),
    #[error("vulkan loading failed: {0}")]
    Loading(#[from] ash::LoadingError),
}

/// Vulkan device context — owns instance, device, queues, and memory allocator.
pub struct GpuContext {
    _entry: ash::Entry,
    instance: ash::Instance,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,
    compute_queue: vk::Queue,
    compute_queue_family: u32,
    command_pool: vk::CommandPool,
    allocator: Mutex<Option<Allocator>>,
}

impl GpuContext {
    /// Initialize Vulkan and select a compute-capable device.
    pub fn new() -> Result<Self, GpuError> {
        let entry = unsafe { ash::Entry::load()? };

        let app_info = vk::ApplicationInfo::default()
            .application_name(c"tazama")
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .engine_name(c"tazama-gpu")
            .engine_version(vk::make_api_version(0, 1, 0, 0))
            .api_version(vk::make_api_version(0, 1, 0, 0));

        let create_info = vk::InstanceCreateInfo::default().application_info(&app_info);

        let instance = unsafe { entry.create_instance(&create_info, None)? };

        let physical_devices = unsafe { instance.enumerate_physical_devices()? };
        if physical_devices.is_empty() {
            return Err(GpuError::NoDevice);
        }

        // Find a device with a compute queue
        let mut selected = None;
        for &pdev in &physical_devices {
            let queue_families =
                unsafe { instance.get_physical_device_queue_family_properties(pdev) };
            for (idx, family) in queue_families.iter().enumerate() {
                if family.queue_flags.contains(vk::QueueFlags::COMPUTE) {
                    selected = Some((pdev, idx as u32));
                    break;
                }
            }
            if selected.is_some() {
                break;
            }
        }

        let (physical_device, compute_queue_family) = selected.ok_or(GpuError::NoDevice)?;

        // Log the selected device
        let props = unsafe { instance.get_physical_device_properties(physical_device) };
        let device_name = unsafe { CStr::from_ptr(props.device_name.as_ptr()) }
            .to_string_lossy()
            .to_string();
        tracing::info!("GPU: {device_name}");

        let queue_priority = [1.0f32];
        let queue_create_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(compute_queue_family)
            .queue_priorities(&queue_priority);

        let queue_create_infos = [queue_create_info];
        let device_create_info =
            vk::DeviceCreateInfo::default().queue_create_infos(&queue_create_infos);

        let device = unsafe { instance.create_device(physical_device, &device_create_info, None)? };

        let compute_queue = unsafe { device.get_device_queue(compute_queue_family, 0) };

        let pool_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(compute_queue_family)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

        let command_pool = unsafe { device.create_command_pool(&pool_info, None)? };

        let allocator = Allocator::new(&AllocatorCreateDesc {
            instance: instance.clone(),
            device: device.clone(),
            physical_device,
            debug_settings: Default::default(),
            buffer_device_address: false,
            allocation_sizes: Default::default(),
        })
        .map_err(|e| GpuError::Allocator(e.to_string()))?;

        tracing::info!("GPU context initialized");

        Ok(Self {
            _entry: entry,
            instance,
            physical_device,
            device,
            compute_queue,
            compute_queue_family,
            command_pool,
            allocator: Mutex::new(Some(allocator)),
        })
    }

    pub fn device(&self) -> &ash::Device {
        &self.device
    }

    pub fn compute_queue(&self) -> vk::Queue {
        self.compute_queue
    }

    pub fn compute_queue_family(&self) -> u32 {
        self.compute_queue_family
    }

    pub fn command_pool(&self) -> vk::CommandPool {
        self.command_pool
    }

    pub fn allocator(&self) -> &Mutex<Option<Allocator>> {
        &self.allocator
    }

    pub fn physical_device(&self) -> vk::PhysicalDevice {
        self.physical_device
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_error_display() {
        let e = GpuError::NoDevice;
        assert_eq!(e.to_string(), "no suitable GPU found");

        let e = GpuError::ShaderCompilation("bad shader".into());
        assert!(e.to_string().contains("bad shader"));

        let e = GpuError::BufferNotMapped;
        assert_eq!(e.to_string(), "buffer not mapped for access");

        let e = GpuError::FrameSource("missing frame".into());
        assert!(e.to_string().contains("missing frame"));

        let e = GpuError::Allocator("out of memory".into());
        assert!(e.to_string().contains("out of memory"));
    }
}

impl Drop for GpuContext {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();
            self.device.destroy_command_pool(self.command_pool, None);
            // Drop allocator before device — take() removes it from the Option
            // so it is dropped here, before the device is destroyed.
            let allocator = self.allocator.get_mut().unwrap_or_else(|e| e.into_inner());
            drop(allocator.take());
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}
