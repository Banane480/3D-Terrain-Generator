use std::ffi::CStr;
use ash::vk;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::Window;

#[derive(Clone, Copy, Debug)]
pub struct QueueFamilyIndices {
    pub graphics_family: u32,
    pub present_family: u32,
}

pub struct VulkanContext {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub surface_loader: ash::khr::surface::Instance,
    pub surface: vk::SurfaceKHR,
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,
    pub queue_family_indices: QueueFamilyIndices,
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,
    pub command_pool: vk::CommandPool,
}

impl VulkanContext {
    pub fn new(window: &Window) -> Result<Self, Box<dyn std::error::Error>> {
        let entry = unsafe { ash::Entry::load()? };

        let app_name = b"Vulkan Procedural World\0";
        let engine_name = b"RustVulkanEngine\0";
        let app_info = vk::ApplicationInfo::default()
            .application_name(unsafe { CStr::from_bytes_with_nul_unchecked(app_name) })
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .engine_name(unsafe { CStr::from_bytes_with_nul_unchecked(engine_name) })
            .engine_version(vk::make_api_version(0, 1, 0, 0))
            .api_version(vk::API_VERSION_1_2);

        // Extensions requises pour le fenetrage de surface
        let display_handle = window.display_handle()?.as_raw();
        let window_handle = window.window_handle()?.as_raw();
        let extension_names = ash_window::enumerate_required_extensions(display_handle)?;

        let instance_create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(extension_names);

        let instance = unsafe { entry.create_instance(&instance_create_info, None)? };
        let surface_loader = ash::khr::surface::Instance::new(&entry, &instance);
        let surface = unsafe {
            ash_window::create_surface(&entry, &instance, display_handle, window_handle, None)?
        };

        // Selection du GPU physique (priorite au GPU dedie)
        let physical_devices = unsafe { instance.enumerate_physical_devices()? };
        let (physical_device, queue_family_indices) = physical_devices
            .into_iter()
            .filter_map(|pdev| {
                let indices = Self::find_queue_families(&instance, &surface_loader, surface, pdev);
                indices.map(|idx| (pdev, idx))
            })
            .max_by_key(|(pdev, _)| {
                let props = unsafe { instance.get_physical_device_properties(*pdev) };
                match props.device_type {
                    vk::PhysicalDeviceType::DISCRETE_GPU => 1000,
                    vk::PhysicalDeviceType::INTEGRATED_GPU => 100,
                    _ => 1,
                }
            })
            .ok_or("Aucun GPU Vulkan compatible trouve")?;

        let props = unsafe { instance.get_physical_device_properties(physical_device) };
        let dev_name = unsafe { CStr::from_ptr(props.device_name.as_ptr()) };
        println!("[Vulkan] GPU selectionne: {:?}", dev_name);

        // Creation du Logical Device
        let mut unique_indices = vec![queue_family_indices.graphics_family];
        if queue_family_indices.graphics_family != queue_family_indices.present_family {
            unique_indices.push(queue_family_indices.present_family);
        }

        let queue_priorities = [1.0f32];
        let queue_create_infos: Vec<_> = unique_indices
            .iter()
            .map(|&idx| {
                vk::DeviceQueueCreateInfo::default()
                    .queue_family_index(idx)
                    .queue_priorities(&queue_priorities)
            })
            .collect();

        let device_extensions = [ash::khr::swapchain::NAME.as_ptr()];
        let device_features = vk::PhysicalDeviceFeatures::default()
            .sampler_anisotropy(true)
            .fill_mode_non_solid(true);

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&device_extensions)
            .enabled_features(&device_features);

        let device = unsafe { instance.create_device(physical_device, &device_create_info, None)? };

        let graphics_queue = unsafe { device.get_device_queue(queue_family_indices.graphics_family, 0) };
        let present_queue = unsafe { device.get_device_queue(queue_family_indices.present_family, 0) };

        // Command pool
        let pool_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue_family_indices.graphics_family)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let command_pool = unsafe { device.create_command_pool(&pool_info, None)? };

        Ok(Self {
            entry,
            instance,
            surface_loader,
            surface,
            physical_device,
            device,
            queue_family_indices,
            graphics_queue,
            present_queue,
            command_pool,
        })
    }

    fn find_queue_families(
        instance: &ash::Instance,
        surface_loader: &ash::khr::surface::Instance,
        surface: vk::SurfaceKHR,
        physical_device: vk::PhysicalDevice,
    ) -> Option<QueueFamilyIndices> {
        let queue_families = unsafe {
            instance.get_physical_device_queue_family_properties(physical_device)
        };

        let mut graphics_family = None;
        let mut present_family = None;

        for (idx, qf) in queue_families.iter().enumerate() {
            let i = idx as u32;
            if qf.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                graphics_family = Some(i);
            }

            let present_support = unsafe {
                surface_loader
                    .get_physical_device_surface_support(physical_device, i, surface)
                    .unwrap_or(false)
            };
            if present_support {
                present_family = Some(i);
            }

            if graphics_family.is_some() && present_family.is_some() {
                break;
            }
        }

        match (graphics_family, present_family) {
            (Some(g), Some(p)) => Some(QueueFamilyIndices {
                graphics_family: g,
                present_family: p,
            }),
            _ => None,
        }
    }
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_command_pool(self.command_pool, None);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.instance.destroy_instance(None);
        }
    }
}
