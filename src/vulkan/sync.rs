use ash::vk;
use crate::vulkan::context::VulkanContext;

pub struct SyncObjects {
    pub image_available_semaphores: Vec<vk::Semaphore>,
    pub render_finished_semaphores: Vec<vk::Semaphore>,
    pub in_flight_fences: Vec<vk::Fence>,
}

impl SyncObjects {
    pub fn new(ctx: &VulkanContext, max_frames: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let semaphore_info = vk::SemaphoreCreateInfo::default();
        let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);

        let mut image_available_semaphores = Vec::with_capacity(max_frames);
        let mut render_finished_semaphores = Vec::with_capacity(max_frames);
        let mut in_flight_fences = Vec::with_capacity(max_frames);

        for _ in 0..max_frames {
            unsafe {
                image_available_semaphores.push(ctx.device.create_semaphore(&semaphore_info, None)?);
                render_finished_semaphores.push(ctx.device.create_semaphore(&semaphore_info, None)?);
                in_flight_fences.push(ctx.device.create_fence(&fence_info, None)?);
            }
        }

        Ok(Self {
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
        })
    }

    pub fn destroy(&mut self, ctx: &VulkanContext) {
        unsafe {
            for sem in self.image_available_semaphores.drain(..) {
                ctx.device.destroy_semaphore(sem, None);
            }
            for sem in self.render_finished_semaphores.drain(..) {
                ctx.device.destroy_semaphore(sem, None);
            }
            for fence in self.in_flight_fences.drain(..) {
                ctx.device.destroy_fence(fence, None);
            }
        }
    }
}
