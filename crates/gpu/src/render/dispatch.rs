use ash::vk;

use crate::buffer::GpuBuffer;
use crate::context::GpuError;

use super::Renderer;

impl Renderer {
    /// Dispatch a compute shader with 2 storage buffer bindings.
    pub(crate) fn dispatch_2buffer(
        &self,
        pipeline: &crate::pipeline::ComputePipeline,
        input: &GpuBuffer,
        output: &GpuBuffer,
        push_constants: &[u8],
        pixel_count: u32,
    ) -> Result<(), GpuError> {
        let device = self.ctx.device();

        // Allocate descriptor set
        let set_layouts = [pipeline.descriptor_set_layout];
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.pipelines.descriptor_pool)
            .set_layouts(&set_layouts);
        let descriptor_set = unsafe { device.allocate_descriptor_sets(&alloc_info)? }[0];

        // Update descriptor set
        let input_info = vk::DescriptorBufferInfo::default()
            .buffer(input.vk_buffer())
            .offset(0)
            .range(vk::WHOLE_SIZE);
        let output_info = vk::DescriptorBufferInfo::default()
            .buffer(output.vk_buffer())
            .offset(0)
            .range(vk::WHOLE_SIZE);

        let input_infos = [input_info];
        let output_infos = [output_info];

        let writes = [
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&input_infos),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(1)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&output_infos),
        ];

        unsafe { device.update_descriptor_sets(&writes, &[]) };

        self.submit_compute(pipeline, descriptor_set, push_constants, pixel_count)?;

        Ok(())
    }

    /// Dispatch a compute shader with 3 storage buffer bindings.
    pub(crate) fn dispatch_3buffer(
        &self,
        pipeline: &crate::pipeline::ComputePipeline,
        buf_a: &GpuBuffer,
        buf_b: &GpuBuffer,
        output: &GpuBuffer,
        push_constants: &[u8],
        pixel_count: u32,
    ) -> Result<(), GpuError> {
        let device = self.ctx.device();

        let set_layouts = [pipeline.descriptor_set_layout];
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.pipelines.descriptor_pool)
            .set_layouts(&set_layouts);
        let descriptor_set = unsafe { device.allocate_descriptor_sets(&alloc_info)? }[0];

        let a_info = vk::DescriptorBufferInfo::default()
            .buffer(buf_a.vk_buffer())
            .offset(0)
            .range(vk::WHOLE_SIZE);
        let b_info = vk::DescriptorBufferInfo::default()
            .buffer(buf_b.vk_buffer())
            .offset(0)
            .range(vk::WHOLE_SIZE);
        let out_info = vk::DescriptorBufferInfo::default()
            .buffer(output.vk_buffer())
            .offset(0)
            .range(vk::WHOLE_SIZE);

        let a_infos = [a_info];
        let b_infos = [b_info];
        let out_infos = [out_info];

        let writes = [
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&a_infos),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(1)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&b_infos),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(2)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&out_infos),
        ];

        unsafe { device.update_descriptor_sets(&writes, &[]) };

        self.submit_compute(pipeline, descriptor_set, push_constants, pixel_count)?;

        Ok(())
    }

    /// Record and submit a compute dispatch, then wait for completion.
    pub(crate) fn submit_compute(
        &self,
        pipeline: &crate::pipeline::ComputePipeline,
        descriptor_set: vk::DescriptorSet,
        push_constants: &[u8],
        pixel_count: u32,
    ) -> Result<(), GpuError> {
        let device = self.ctx.device();

        unsafe {
            device
                .reset_command_buffer(self.command_buffer, vk::CommandBufferResetFlags::empty())?;

            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            device.begin_command_buffer(self.command_buffer, &begin_info)?;

            device.cmd_bind_pipeline(
                self.command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                pipeline.pipeline,
            );

            device.cmd_bind_descriptor_sets(
                self.command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                pipeline.layout,
                0,
                &[descriptor_set],
                &[],
            );

            device.cmd_push_constants(
                self.command_buffer,
                pipeline.layout,
                vk::ShaderStageFlags::COMPUTE,
                0,
                push_constants,
            );

            let group_count = pixel_count.div_ceil(256);
            device.cmd_dispatch(self.command_buffer, group_count, 1, 1);

            // Memory barrier for compute → compute/transfer
            let barrier = vk::MemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::SHADER_WRITE)
                .dst_access_mask(
                    vk::AccessFlags::SHADER_READ
                        | vk::AccessFlags::TRANSFER_READ
                        | vk::AccessFlags::HOST_READ,
                );
            device.cmd_pipeline_barrier(
                self.command_buffer,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::PipelineStageFlags::COMPUTE_SHADER
                    | vk::PipelineStageFlags::TRANSFER
                    | vk::PipelineStageFlags::HOST,
                vk::DependencyFlags::empty(),
                &[barrier],
                &[],
                &[],
            );

            device.end_command_buffer(self.command_buffer)?;

            device.reset_fences(&[self.fence])?;

            let command_buffers = [self.command_buffer];
            let submit_info = vk::SubmitInfo::default().command_buffers(&command_buffers);
            device.queue_submit(self.ctx.compute_queue(), &[submit_info], self.fence)?;

            device.wait_for_fences(&[self.fence], true, u64::MAX)?;
        }

        Ok(())
    }

    /// Clear a buffer to zero.
    pub(crate) fn clear_buffer(&self, buffer: &GpuBuffer, size: u64) -> Result<(), GpuError> {
        let device = self.ctx.device();
        unsafe {
            device
                .reset_command_buffer(self.command_buffer, vk::CommandBufferResetFlags::empty())?;
            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            device.begin_command_buffer(self.command_buffer, &begin_info)?;
            device.cmd_fill_buffer(self.command_buffer, buffer.vk_buffer(), 0, size, 0);

            let barrier = vk::MemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ | vk::AccessFlags::SHADER_WRITE);
            device.cmd_pipeline_barrier(
                self.command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::DependencyFlags::empty(),
                &[barrier],
                &[],
                &[],
            );

            device.end_command_buffer(self.command_buffer)?;
            device.reset_fences(&[self.fence])?;
            let command_buffers = [self.command_buffer];
            let submit_info = vk::SubmitInfo::default().command_buffers(&command_buffers);
            device.queue_submit(self.ctx.compute_queue(), &[submit_info], self.fence)?;
            device.wait_for_fences(&[self.fence], true, u64::MAX)?;
        }
        Ok(())
    }

    /// Copy data between two buffers.
    pub(crate) fn copy_buffer(
        &self,
        src: &GpuBuffer,
        dst: &GpuBuffer,
        size: u64,
    ) -> Result<(), GpuError> {
        let device = self.ctx.device();
        unsafe {
            device
                .reset_command_buffer(self.command_buffer, vk::CommandBufferResetFlags::empty())?;
            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            device.begin_command_buffer(self.command_buffer, &begin_info)?;

            let region = vk::BufferCopy::default().size(size);
            device.cmd_copy_buffer(
                self.command_buffer,
                src.vk_buffer(),
                dst.vk_buffer(),
                &[region],
            );

            let barrier = vk::MemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::HOST_READ);
            device.cmd_pipeline_barrier(
                self.command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::HOST,
                vk::DependencyFlags::empty(),
                &[barrier],
                &[],
                &[],
            );

            device.end_command_buffer(self.command_buffer)?;
            device.reset_fences(&[self.fence])?;
            let command_buffers = [self.command_buffer];
            let submit_info = vk::SubmitInfo::default().command_buffers(&command_buffers);
            device.queue_submit(self.ctx.compute_queue(), &[submit_info], self.fence)?;
            device.wait_for_fences(&[self.fence], true, u64::MAX)?;
        }
        Ok(())
    }
}
