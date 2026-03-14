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
        if !spirv.len().is_multiple_of(4) {
            return Err(GpuError::ShaderCompilation(
                "SPIR-V bytecode is not 4-byte aligned".into(),
            ));
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_shaders_are_4byte_aligned() {
        assert!(COLOR_GRADE_SPV.len().is_multiple_of(4));
        assert!(COMPOSITE_SPV.len().is_multiple_of(4));
        assert!(CROP_SPV.len().is_multiple_of(4));
        assert!(DISSOLVE_SPV.len().is_multiple_of(4));
        assert!(WIPE_SPV.len().is_multiple_of(4));
        assert!(FADE_SPV.len().is_multiple_of(4));
    }

    #[test]
    fn embedded_shaders_are_not_empty() {
        assert!(!COLOR_GRADE_SPV.is_empty());
        assert!(!COMPOSITE_SPV.is_empty());
        assert!(!CROP_SPV.is_empty());
        assert!(!DISSOLVE_SPV.is_empty());
        assert!(!WIPE_SPV.is_empty());
        assert!(!FADE_SPV.is_empty());
    }

    #[test]
    fn embedded_shaders_have_spirv_magic() {
        // SPIR-V magic number is 0x07230203
        fn check_magic(spv: &[u8]) {
            let magic = u32::from_le_bytes([spv[0], spv[1], spv[2], spv[3]]);
            assert_eq!(magic, 0x07230203, "invalid SPIR-V magic number");
        }
        check_magic(COLOR_GRADE_SPV);
        check_magic(COMPOSITE_SPV);
        check_magic(CROP_SPV);
        check_magic(DISSOLVE_SPV);
        check_magic(WIPE_SPV);
        check_magic(FADE_SPV);
    }
}
