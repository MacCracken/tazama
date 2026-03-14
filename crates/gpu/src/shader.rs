use ash::vk;

use crate::context::GpuError;

pub static COLOR_GRADE_SPV: &[u8] = include_bytes!("../shaders/color_grade.spv");
pub static COMPOSITE_SPV: &[u8] = include_bytes!("../shaders/composite.spv");
pub static CROP_SPV: &[u8] = include_bytes!("../shaders/crop.spv");
pub static DISSOLVE_SPV: &[u8] = include_bytes!("../shaders/dissolve.spv");
pub static WIPE_SPV: &[u8] = include_bytes!("../shaders/wipe.spv");
pub static FADE_SPV: &[u8] = include_bytes!("../shaders/fade.spv");

/// A loaded Vulkan shader module.
pub struct ShaderModule {
    pub(crate) module: vk::ShaderModule,
}

impl ShaderModule {
    /// Create a shader module from SPIR-V bytes.
    pub fn from_spirv(device: &ash::Device, spirv: &[u8]) -> Result<Self, GpuError> {
        // Align to u32
        let code: Vec<u32> = spirv
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();

        let create_info = vk::ShaderModuleCreateInfo::default().code(&code);

        let module = unsafe { device.create_shader_module(&create_info, None)? };

        Ok(Self { module })
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_shader_module(self.module, None);
        }
    }
}
