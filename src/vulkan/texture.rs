use ash::vk;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc};
use gpu_allocator::MemoryLocation;

use crate::vulkan::allocator::GpuAllocator;
use crate::vulkan::context::VulkanContext;

pub struct VulkanTextureArray {
    pub image: vk::Image,
    pub allocation: Option<Allocation>,
    pub image_view: vk::ImageView,
    pub sampler: vk::Sampler,
}

impl VulkanTextureArray {
    pub fn load_terrain_textures(
        ctx: &VulkanContext,
        allocator: &GpuAllocator,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        const TEX_SIZE: u32 = 1024;
        const NUM_LAYERS: u32 = 8;
        let layer_byte_size = (TEX_SIZE * TEX_SIZE * 4) as usize;
        let total_byte_size = (layer_byte_size * NUM_LAYERS as usize) as vk::DeviceSize;

        println!("[Texture] Chargement des 8 textures terrain/vegetation 1K...");

        // Inclusion directe dans le binaire pour garantir 100% de disponibilite quel que soit l'endroit ou est lance le .exe
        let raw_images: [&[u8]; 8] = [
            include_bytes!("../../assets/textures/sand/Ground054_1K-JPG_Color.jpg"),
            include_bytes!("../../assets/textures/grass/Ground037_1K-JPG_Color.jpg"),
            include_bytes!("../../assets/textures/rock/Rock020_1K-JPG_Color.jpg"),
            include_bytes!("../../assets/textures/snow/Snow006_1K-JPG_Color.jpg"),
            include_bytes!("../../assets/models/vegetation/quaternius/glTF/Bark_NormalTree.png"),
            include_bytes!("../../assets/models/vegetation/quaternius/glTF/Leaf_Pine_C.png"),
            include_bytes!("../../assets/models/vegetation/quaternius/glTF/Leaves_NormalTree_C.png"),
            include_bytes!("../../assets/models/vegetation/quaternius/glTF/Leaves_TwistedTree_C.png"),
        ];

        let mut pixel_data = Vec::with_capacity(layer_byte_size * NUM_LAYERS as usize);

        for (idx, bytes) in raw_images.iter().enumerate() {
            println!("[Texture] Decodage couche {} (1024x1024)...", idx);
            let img = image::load_from_memory(bytes)?;
            let resized = if img.width() != TEX_SIZE || img.height() != TEX_SIZE {
                img.resize_exact(TEX_SIZE, TEX_SIZE, image::imageops::FilterType::Triangle)
            } else {
                img
            };
            let rgba = resized.to_rgba8();
            pixel_data.extend_from_slice(&rgba);
        }

        println!("[Texture] Toutes les textures decodees avec succes ({} Mo)", pixel_data.len() / (1024 * 1024));

        // 1. Staging Buffer CPU->GPU
        let (staging_buffer, staging_allocation) = allocator.allocate_buffer(
            &ctx.device,
            "Texture Array Staging",
            total_byte_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            MemoryLocation::CpuToGpu,
        )?;

        if let Some(mapped_ptr) = staging_allocation.mapped_ptr() {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    pixel_data.as_ptr(),
                    mapped_ptr.as_ptr() as *mut u8,
                    pixel_data.len(),
                );
            }
        }

        // 2. Texture VkImage 2D Array
        let format = vk::Format::R8G8B8A8_SRGB;
        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(vk::Extent3D {
                width: TEX_SIZE,
                height: TEX_SIZE,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(NUM_LAYERS)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        let image = unsafe { ctx.device.create_image(&image_info, None)? };
        let requirements = unsafe { ctx.device.get_image_memory_requirements(image) };

        let allocation = allocator.allocator.lock().unwrap().allocate(&AllocationCreateDesc {
            name: "Terrain Texture Array",
            requirements,
            location: MemoryLocation::GpuOnly,
            linear: false,
            allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
        })?;

        unsafe {
            ctx.device.bind_image_memory(image, allocation.memory(), allocation.offset())?;
        }

        // 3. Command Buffer pour transition de layout et copie VRAM
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(ctx.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let cmd = unsafe { ctx.device.allocate_command_buffers(&alloc_info)?[0] };

        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            ctx.device.begin_command_buffer(cmd, &begin_info)?;

            // Transition: UNDEFINED -> TRANSFER_DST_OPTIMAL
            let barrier_to_dst = [vk::ImageMemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::empty())
                .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .image(image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: NUM_LAYERS,
                })];

            ctx.device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &barrier_to_dst,
            );

            // Copie de chaque couche
            let mut regions = Vec::with_capacity(NUM_LAYERS as usize);
            for layer in 0..NUM_LAYERS {
                regions.push(
                    vk::BufferImageCopy::default()
                        .buffer_offset((layer as usize * layer_byte_size) as vk::DeviceSize)
                        .buffer_row_length(0)
                        .buffer_image_height(0)
                        .image_subresource(vk::ImageSubresourceLayers {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            mip_level: 0,
                            base_array_layer: layer,
                            layer_count: 1,
                        })
                        .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
                        .image_extent(vk::Extent3D {
                            width: TEX_SIZE,
                            height: TEX_SIZE,
                            depth: 1,
                        }),
                );
            }

            ctx.device.cmd_copy_buffer_to_image(
                cmd,
                staging_buffer,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &regions,
            );

            // Transition: TRANSFER_DST_OPTIMAL -> SHADER_READ_ONLY_OPTIMAL
            let barrier_to_read = [vk::ImageMemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ)
                .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image(image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: NUM_LAYERS,
                })];

            ctx.device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &barrier_to_read,
            );

            ctx.device.end_command_buffer(cmd)?;

            let cmds = [cmd];
            let submit_info = [vk::SubmitInfo::default().command_buffers(&cmds)];
            ctx.device.queue_submit(ctx.graphics_queue, &submit_info, vk::Fence::null())?;
            ctx.device.queue_wait_idle(ctx.graphics_queue)?;
            ctx.device.free_command_buffers(ctx.command_pool, &[cmd]);
        }

        allocator.destroy_buffer(&ctx.device, staging_buffer, staging_allocation);

        // 4. ImageView 2D Array
        let view_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D_ARRAY)
            .format(format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: NUM_LAYERS,
            });
        let image_view = unsafe { ctx.device.create_image_view(&view_info, None)? };

        // 5. Sampler anisotrope
        let sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::REPEAT)
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .mip_lod_bias(0.0)
            .anisotropy_enable(true)
            .max_anisotropy(16.0)
            .compare_enable(false)
            .min_lod(0.0)
            .max_lod(0.0);
        let sampler = unsafe { ctx.device.create_sampler(&sampler_info, None)? };

        Ok(Self {
            image,
            allocation: Some(allocation),
            image_view,
            sampler,
        })
    }

    pub fn destroy(&mut self, ctx: &VulkanContext, allocator: &GpuAllocator) {
        unsafe {
            ctx.device.destroy_sampler(self.sampler, None);
            ctx.device.destroy_image_view(self.image_view, None);
            ctx.device.destroy_image(self.image, None);
            if let Some(alloc) = self.allocation.take() {
                allocator.allocator.lock().unwrap().free(alloc).ok();
            }
        }
    }
}
