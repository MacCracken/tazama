use ash::vk;
use gpu_allocator::MemoryLocation;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme};

use crate::context::{GpuContext, GpuError};

/// A GPU buffer backed by gpu-allocator.
pub struct GpuBuffer {
    pub(crate) buffer: vk::Buffer,
    pub(crate) allocation: Option<Allocation>,
    pub size: u64,
}

impl GpuBuffer {
    /// Create a new GPU buffer.
    pub fn new(
        ctx: &GpuContext,
        size: u64,
        usage: vk::BufferUsageFlags,
        location: MemoryLocation,
        name: &str,
    ) -> Result<Self, GpuError> {
        let buffer_info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe { ctx.device().create_buffer(&buffer_info, None)? };
        let requirements = unsafe { ctx.device().get_buffer_memory_requirements(buffer) };

        let allocation = ctx
            .allocator()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_mut()
            .ok_or_else(|| GpuError::Allocator("allocator already destroyed".into()))?
            .allocate(&AllocationCreateDesc {
                name,
                requirements,
                location,
                linear: true,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            })
            .map_err(|e| GpuError::Allocator(e.to_string()))?;

        unsafe {
            ctx.device()
                .bind_buffer_memory(buffer, allocation.memory(), allocation.offset())?;
        }

        Ok(Self {
            buffer,
            allocation: Some(allocation),
            size,
        })
    }

    /// Write data to a CPU-visible buffer.
    pub fn write(&mut self, data: &[u8]) -> Result<(), GpuError> {
        let alloc = self.allocation.as_mut().ok_or(GpuError::BufferNotMapped)?;
        let mapped = alloc.mapped_slice_mut().ok_or(GpuError::BufferNotMapped)?;
        mapped[..data.len()].copy_from_slice(data);
        Ok(())
    }

    /// Read data from a CPU-visible buffer.
    pub fn read(&self, len: usize) -> Result<&[u8], GpuError> {
        let alloc = self.allocation.as_ref().ok_or(GpuError::BufferNotMapped)?;
        let mapped = alloc.mapped_slice().ok_or(GpuError::BufferNotMapped)?;
        Ok(&mapped[..len])
    }

    /// Destroy the buffer, freeing its memory.
    pub fn destroy(mut self, ctx: &GpuContext) {
        if let Some(allocation) = self.allocation.take()
            && let Some(alloc) = ctx
                .allocator()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .as_mut()
        {
            let _ = alloc.free(allocation);
        }
        unsafe {
            ctx.device().destroy_buffer(self.buffer, None);
        }
    }

    /// Compute the buffer size for an RGBA frame.
    pub fn frame_buffer_size(width: u32, height: u32) -> u64 {
        width as u64 * height as u64 * 4
    }

    pub fn vk_buffer(&self) -> vk::Buffer {
        self.buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_buffer_size_1080p() {
        assert_eq!(GpuBuffer::frame_buffer_size(1920, 1080), 1920 * 1080 * 4);
    }

    #[test]
    fn frame_buffer_size_4k() {
        assert_eq!(GpuBuffer::frame_buffer_size(3840, 2160), 3840 * 2160 * 4);
    }

    #[test]
    fn frame_buffer_size_zero() {
        assert_eq!(GpuBuffer::frame_buffer_size(0, 0), 0);
        assert_eq!(GpuBuffer::frame_buffer_size(1920, 0), 0);
        assert_eq!(GpuBuffer::frame_buffer_size(0, 1080), 0);
    }
}
