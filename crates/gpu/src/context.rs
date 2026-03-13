use thiserror::Error;

#[derive(Debug, Error)]
pub enum GpuError {
    #[error("vulkan error: {0}")]
    Vulkan(String),
    #[error("no suitable GPU found")]
    NoDevice,
    #[error("shader compilation failed: {0}")]
    ShaderCompilation(String),
}

/// Vulkan device context — owns instance, device, and queues.
pub struct GpuContext {
    // TODO: ash::Instance, ash::Device, queue families
    _private: (),
}

impl GpuContext {
    /// Initialize Vulkan and select a compute-capable device.
    pub fn new() -> Result<Self, GpuError> {
        // TODO: Vulkan initialization via ash
        tracing::info!("GPU context initialized (stub)");
        Ok(Self { _private: () })
    }
}
