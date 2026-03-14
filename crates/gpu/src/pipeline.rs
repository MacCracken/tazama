use ash::vk;
use bytemuck::{Pod, Zeroable};

use crate::context::{GpuContext, GpuError};
use crate::shader::{self, ShaderModule};

// --- Push constant structs ---

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct ColorGradePush {
    pub width: u32,
    pub height: u32,
    pub brightness: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub temperature: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct CompositePush {
    pub width: u32,
    pub height: u32,
    pub opacity: f32,
    pub _pad: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct CropPush {
    pub src_width: u32,
    pub src_height: u32,
    pub dst_width: u32,
    pub dst_height: u32,
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct TransitionPush {
    pub width: u32,
    pub height: u32,
    pub progress: f32,
    pub _pad: u32,
}

// --- Pipeline types ---

/// A single Vulkan compute pipeline with its layout and descriptor set layout.
pub struct ComputePipeline {
    pub(crate) pipeline: vk::Pipeline,
    pub(crate) layout: vk::PipelineLayout,
    pub(crate) descriptor_set_layout: vk::DescriptorSetLayout,
}

impl ComputePipeline {
    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.layout, None);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }
}

/// All compute pipelines needed for rendering.
pub struct PipelineCache {
    pub color_grade: ComputePipeline,
    pub crop: ComputePipeline,
    pub composite: ComputePipeline,
    pub dissolve: ComputePipeline,
    pub wipe: ComputePipeline,
    pub fade: ComputePipeline,
    pub descriptor_pool: vk::DescriptorPool,
}

impl PipelineCache {
    /// Create all 6 compute pipelines and descriptor pool.
    pub fn new(ctx: &GpuContext) -> Result<Self, GpuError> {
        let device = ctx.device();

        // Pool: generous upper bound for descriptor sets
        let pool_sizes = [vk::DescriptorPoolSize {
            ty: vk::DescriptorType::STORAGE_BUFFER,
            descriptor_count: 64,
        }];
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(32)
            .pool_sizes(&pool_sizes);
        let descriptor_pool = unsafe { device.create_descriptor_pool(&pool_info, None)? };

        let color_grade = create_pipeline(
            device,
            shader::COLOR_GRADE_SPV,
            2, // in, out
            std::mem::size_of::<ColorGradePush>() as u32,
        )?;

        let crop = create_pipeline(
            device,
            shader::CROP_SPV,
            2,
            std::mem::size_of::<CropPush>() as u32,
        )?;

        let composite = create_pipeline(
            device,
            shader::COMPOSITE_SPV,
            3, // base, overlay, out
            std::mem::size_of::<CompositePush>() as u32,
        )?;

        let dissolve = create_pipeline(
            device,
            shader::DISSOLVE_SPV,
            3,
            std::mem::size_of::<TransitionPush>() as u32,
        )?;

        let wipe = create_pipeline(
            device,
            shader::WIPE_SPV,
            3,
            std::mem::size_of::<TransitionPush>() as u32,
        )?;

        let fade = create_pipeline(
            device,
            shader::FADE_SPV,
            2,
            std::mem::size_of::<TransitionPush>() as u32,
        )?;

        Ok(Self {
            color_grade,
            crop,
            composite,
            dissolve,
            wipe,
            fade,
            descriptor_pool,
        })
    }

    pub fn destroy(&self, device: &ash::Device) {
        self.color_grade.destroy(device);
        self.crop.destroy(device);
        self.composite.destroy(device);
        self.dissolve.destroy(device);
        self.wipe.destroy(device);
        self.fade.destroy(device);
        unsafe {
            device.destroy_descriptor_pool(self.descriptor_pool, None);
        }
    }
}

/// Create a compute pipeline with N storage buffer bindings and a push constant range.
fn create_pipeline(
    device: &ash::Device,
    spirv: &[u8],
    binding_count: u32,
    push_constant_size: u32,
) -> Result<ComputePipeline, GpuError> {
    let shader_module = ShaderModule::from_spirv(device, spirv)?;

    // Descriptor set layout: N storage buffers
    let bindings: Vec<vk::DescriptorSetLayoutBinding> = (0..binding_count)
        .map(|i| {
            vk::DescriptorSetLayoutBinding::default()
                .binding(i)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE)
        })
        .collect();

    let ds_layout_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
    let descriptor_set_layout =
        unsafe { device.create_descriptor_set_layout(&ds_layout_info, None)? };

    let push_range = vk::PushConstantRange::default()
        .stage_flags(vk::ShaderStageFlags::COMPUTE)
        .offset(0)
        .size(push_constant_size);

    let push_ranges = [push_range];
    let set_layouts = [descriptor_set_layout];
    let layout_info = vk::PipelineLayoutCreateInfo::default()
        .set_layouts(&set_layouts)
        .push_constant_ranges(&push_ranges);

    let layout = unsafe { device.create_pipeline_layout(&layout_info, None)? };

    let stage = vk::PipelineShaderStageCreateInfo::default()
        .stage(vk::ShaderStageFlags::COMPUTE)
        .module(shader_module.module)
        .name(c"main");

    let pipeline_info = vk::ComputePipelineCreateInfo::default()
        .stage(stage)
        .layout(layout);

    let pipelines = unsafe {
        device
            .create_compute_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
            .map_err(|(_, e)| e)?
    };

    shader_module.destroy(device);

    Ok(ComputePipeline {
        pipeline: pipelines[0],
        layout,
        descriptor_set_layout,
    })
}
