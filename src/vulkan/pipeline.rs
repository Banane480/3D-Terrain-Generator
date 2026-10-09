use std::io::Cursor;
use ash::vk;
use crate::terrain::mesh::TerrainVertex;
use crate::vulkan::buffer::GpuBuffer;
use crate::vulkan::context::VulkanContext;
use crate::vulkan::texture::VulkanTextureArray;

pub struct VulkanPipeline {
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    pub pipeline_layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
}

impl VulkanPipeline {
    pub fn new(
        ctx: &VulkanContext,
        render_pass: vk::RenderPass,
        uniform_buffers: &[GpuBuffer],
        textures: &VulkanTextureArray,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // 1. Descriptor Set Layout
        // Binding 0: Camera Uniform (Vertex + Fragment)
        // Binding 1: Texture 2D Array (Fragment)
        // Binding 2: Sampler (Fragment)
        let layout_bindings = [
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT),
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
            vk::DescriptorSetLayoutBinding::default()
                .binding(2)
                .descriptor_type(vk::DescriptorType::SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
        ];

        let layout_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&layout_bindings);
        let descriptor_set_layout = unsafe {
            ctx.device.create_descriptor_set_layout(&layout_info, None)?
        };

        // 2. Pipeline Layout
        let set_layouts = [descriptor_set_layout];
        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&set_layouts);
        let pipeline_layout = unsafe {
            ctx.device.create_pipeline_layout(&pipeline_layout_info, None)?
        };

        // 3. Modules Shaders compiles
        let vert_spv_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/terrain_vert.spv"));
        let frag_spv_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/terrain_frag.spv"));

        let mut vert_cursor = Cursor::new(&vert_spv_bytes[..]);
        let mut frag_cursor = Cursor::new(&frag_spv_bytes[..]);

        let vert_words = ash::util::read_spv(&mut vert_cursor)?;
        let frag_words = ash::util::read_spv(&mut frag_cursor)?;

        let vert_module_info = vk::ShaderModuleCreateInfo::default().code(&vert_words);
        let frag_module_info = vk::ShaderModuleCreateInfo::default().code(&frag_words);

        let vert_module = unsafe { ctx.device.create_shader_module(&vert_module_info, None)? };
        let frag_module = unsafe { ctx.device.create_shader_module(&frag_module_info, None)? };

        let entry_point = std::ffi::CStr::from_bytes_with_nul(b"main\0")?;

        let shader_stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vert_module)
                .name(entry_point),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(frag_module)
                .name(entry_point),
        ];

        // 4. Vertex Input State
        let binding_desc = [TerrainVertex::binding_description()];
        let attribute_descs = TerrainVertex::attribute_descriptions();

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&binding_desc)
            .vertex_attribute_descriptions(&attribute_descs);

        // 5. Input Assembly
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        // 6. Viewport et Scissor dynamiques
        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_info = vk::PipelineDynamicStateCreateInfo::default()
            .dynamic_states(&dynamic_states);

        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);

        // 7. Rasterizer (cull_mode None pour visibilite totale sous tous les angles)
        let rasterizer = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false);

        // 8. Multisampling
        let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        // 9. Depth and Stencil State
        let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false);

        // 10. Color Blending
        let color_blend_attachment = [vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .blend_enable(false)];

        let color_blending = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .attachments(&color_blend_attachment);

        // 11. Creation de la Graphics Pipeline
        let pipeline_info = [vk::GraphicsPipelineCreateInfo::default()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterizer)
            .multisample_state(&multisampling)
            .depth_stencil_state(&depth_stencil)
            .color_blend_state(&color_blending)
            .dynamic_state(&dynamic_state_info)
            .layout(pipeline_layout)
            .render_pass(render_pass)
            .subpass(0)];

        let pipeline = unsafe {
            ctx.device
                .create_graphics_pipelines(vk::PipelineCache::null(), &pipeline_info, None)
                .map_err(|(_, e)| e)?[0]
        };

        unsafe {
            ctx.device.destroy_shader_module(vert_module, None);
            ctx.device.destroy_shader_module(frag_module, None);
        }

        // 12. Descriptor Pool et Descriptor Sets pour les Frames in Flight
        let num_frames = uniform_buffers.len() as u32;
        let pool_sizes = [
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(num_frames),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count(num_frames),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::SAMPLER)
                .descriptor_count(num_frames),
        ];

        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&pool_sizes)
            .max_sets(num_frames);

        let descriptor_pool = unsafe { ctx.device.create_descriptor_pool(&pool_info, None)? };

        let layouts = vec![descriptor_set_layout; uniform_buffers.len()];
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&layouts);

        let descriptor_sets = unsafe { ctx.device.allocate_descriptor_sets(&alloc_info)? };

        for (i, buffer) in uniform_buffers.iter().enumerate() {
            let buffer_info = [vk::DescriptorBufferInfo::default()
                .buffer(buffer.buffer)
                .offset(0)
                .range(buffer.size)];

            let image_info = [vk::DescriptorImageInfo::default()
                .image_view(textures.image_view)
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];

            let sampler_info = [vk::DescriptorImageInfo::default()
                .sampler(textures.sampler)];

            let descriptor_writes = [
                // Binding 0: Camera UBO
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_sets[i])
                    .dst_binding(0)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(&buffer_info),
                // Binding 1: Texture 2D Array
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_sets[i])
                    .dst_binding(1)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                    .image_info(&image_info),
                // Binding 2: Sampler
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_sets[i])
                    .dst_binding(2)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::SAMPLER)
                    .image_info(&sampler_info),
            ];

            unsafe {
                ctx.device.update_descriptor_sets(&descriptor_writes, &[]);
            }
        }

        Ok(Self {
            descriptor_set_layout,
            descriptor_pool,
            descriptor_sets,
            pipeline_layout,
            pipeline,
        })
    }

    pub fn destroy(&mut self, ctx: &VulkanContext) {
        unsafe {
            ctx.device.destroy_descriptor_pool(self.descriptor_pool, None);
            ctx.device.destroy_pipeline(self.pipeline, None);
            ctx.device.destroy_pipeline_layout(self.pipeline_layout, None);
            ctx.device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }
}
