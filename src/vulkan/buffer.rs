use ash::vk;
use bytemuck::Pod;
use gpu_allocator::vulkan::Allocation;
use gpu_allocator::MemoryLocation;
use crate::vulkan::allocator::GpuAllocator;
use crate::vulkan::context::VulkanContext;

pub struct GpuBuffer {
    pub buffer: vk::Buffer,
    pub allocation: Option<Allocation>,
    pub size: vk::DeviceSize,
}

impl GpuBuffer {
    /// Crée un Uniform Buffer mappé CPU->GPU pour mise à jour à chaque frame
    pub fn create_uniform_buffer(
        ctx: &VulkanContext,
        allocator: &GpuAllocator,
        name: &'static str,
        size: vk::DeviceSize,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let (buffer, allocation) = allocator.allocate_buffer(
            &ctx.device,
            name,
            size,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
        )?;

        Ok(Self {
            buffer,
            allocation: Some(allocation),
            size,
        })
    }

    /// Met à jour les données dans le buffer mappé
    pub fn update_data<T: Pod>(&mut self, data: &T) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref mut alloc) = self.allocation {
            if let Some(mapped_ptr) = alloc.mapped_ptr() {
                let bytes = bytemuck::bytes_of(data);
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        bytes.as_ptr(),
                        mapped_ptr.as_ptr() as *mut u8,
                        bytes.len(),
                    );
                }
                return Ok(());
            }
        }
        Err("Allocation non mappable en memoire CPU".into())
    }

    /// Crée un buffer GPU haute performance (VRAM GpuOnly) initialisé via un Staging Buffer
    pub fn create_device_local_with_data<T: Pod>(
        ctx: &VulkanContext,
        allocator: &GpuAllocator,
        name: &'static str,
        usage: vk::BufferUsageFlags,
        data: &[T],
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let byte_size = (std::mem::size_of::<T>() * data.len()) as vk::DeviceSize;

        // 1. Staging Buffer CPU->GPU
        let (staging_buffer, staging_allocation) = allocator.allocate_buffer(
            &ctx.device,
            "Staging Buffer",
            byte_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            MemoryLocation::CpuToGpu,
        )?;

        // Copie des octets dans le staging buffer
        if let Some(mapped_ptr) = staging_allocation.mapped_ptr() {
            let slice = bytemuck::cast_slice(data);
            unsafe {
                std::ptr::copy_nonoverlapping(
                    slice.as_ptr(),
                    mapped_ptr.as_ptr() as *mut u8,
                    slice.len(),
                );
            }
        }

        // 2. Buffer de destination VRAM GpuOnly
        let (dst_buffer, dst_allocation) = allocator.allocate_buffer(
            &ctx.device,
            name,
            byte_size,
            usage | vk::BufferUsageFlags::TRANSFER_DST,
            MemoryLocation::GpuOnly,
        )?;

        // 3. Copie GPU via One-Time Command Buffer
        Self::copy_buffer(ctx, staging_buffer, dst_buffer, byte_size)?;

        // 4. Nettoyage du staging buffer
        allocator.destroy_buffer(&ctx.device, staging_buffer, staging_allocation);

        Ok(Self {
            buffer: dst_buffer,
            allocation: Some(dst_allocation),
            size: byte_size,
        })
    }

    fn copy_buffer(
        ctx: &VulkanContext,
        src: vk::Buffer,
        dst: vk::Buffer,
        size: vk::DeviceSize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(ctx.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let command_buffers = unsafe { ctx.device.allocate_command_buffers(&alloc_info)? };
        let cmd = command_buffers[0];

        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            ctx.device.begin_command_buffer(cmd, &begin_info)?;

            let copy_region = [vk::BufferCopy::default().size(size)];
            ctx.device.cmd_copy_buffer(cmd, src, dst, &copy_region);

            ctx.device.end_command_buffer(cmd)?;

            let submit_info = [vk::SubmitInfo::default()
                .command_buffers(&command_buffers)];

            ctx.device.queue_submit(ctx.graphics_queue, &submit_info, vk::Fence::null())?;
            ctx.device.queue_wait_idle(ctx.graphics_queue)?;

            ctx.device.free_command_buffers(ctx.command_pool, &command_buffers);
        }

        Ok(())
    }

    pub fn destroy(&mut self, ctx: &VulkanContext, allocator: &GpuAllocator) {
        if let Some(alloc) = self.allocation.take() {
            allocator.destroy_buffer(&ctx.device, self.buffer, alloc);
        }
    }
}
