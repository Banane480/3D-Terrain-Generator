mod camera;
mod environment;
mod input;
mod terrain;
mod vegetation;
mod vulkan;

use std::{path::Path, time::Instant};
use ash::vk;
use glam::Vec3;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, DeviceId, ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowAttributes, WindowId};

use crate::camera::{Camera, CameraUniform};
use crate::environment::{SkyMesh, WaterMesh};
use crate::input::InputState;
use crate::terrain::TerrainWorld;
use crate::vegetation::{load_quaternius_lods, LoadedVegetation, VegetationInstance, VegetationSpawner};
use crate::vulkan::{
    GpuAllocator, GpuBuffer, SkyPipeline, SyncObjects, VegetationPipeline,
    VulkanContext, VulkanPipeline, VulkanSwapchain, VulkanTextureArray, WaterPipeline,
};

const MAX_FRAMES_IN_FLIGHT: usize = 2;

struct VegetationLodGpu {
    index_buffer: GpuBuffer,
    index_count: u32,
    instance_buffer: Option<GpuBuffer>,
    instance_count: u32,
}

struct VegetationGpuMesh {
    vertex_buffer: GpuBuffer,
    lods: [VegetationLodGpu; 3],
}

impl VegetationGpuMesh {
    unsafe fn draw(&self, context: &VulkanContext, cmd: vk::CommandBuffer) {
        for lod in &self.lods {
            let Some(instance_buffer) = &lod.instance_buffer else { continue };
            context.device.cmd_bind_vertex_buffers(
                cmd, 0, &[self.vertex_buffer.buffer, instance_buffer.buffer], &[0, 0],
            );
            context.device.cmd_bind_index_buffer(cmd, lod.index_buffer.buffer, 0, vk::IndexType::UINT32);
            context.device.cmd_draw_indexed(cmd, lod.index_count, lod.instance_count, 0, 0, 0);
        }
    }

    fn destroy(&mut self, context: &VulkanContext, allocator: &GpuAllocator) {
        self.vertex_buffer.destroy(context, allocator);
        for lod in &mut self.lods {
            lod.index_buffer.destroy(context, allocator);
            if let Some(mut buffer) = lod.instance_buffer.take() {
                buffer.destroy(context, allocator);
            }
        }
    }
}

fn upload_vegetation(
    context: &VulkanContext,
    allocator: &GpuAllocator,
    name: &'static str,
    loaded: LoadedVegetation,
    instances: &[VegetationInstance],
    thresholds: [f32; 2],
) -> Result<VegetationGpuMesh, Box<dyn std::error::Error>> {
    let vertex_buffer = GpuBuffer::create_device_local_with_data(
        context, allocator, name, vk::BufferUsageFlags::VERTEX_BUFFER, &loaded.vertices,
    )?;
    let mut split: [Vec<VegetationInstance>; 3] = std::array::from_fn(|_| Vec::new());
    for instance in instances {
        let x = instance.model_col3[0];
        let z = instance.model_col3[2];
        // shortcut: LODs stay centered on this finite world's origin, rebuild buckets for streamed worlds.
        let distance = (x * x + z * z).sqrt();
        let lod = if distance < thresholds[0] { 0 } else if distance < thresholds[1] { 1 } else { 2 };
        split[lod].push(*instance);
    }
    let mut index_sets = loaded.lod_indices.into_iter();
    let mut instance_sets = split.into_iter();
    let mut lods = Vec::with_capacity(3);
    for lod in 0..3 {
        let indices = index_sets.next().unwrap();
        let instances = instance_sets.next().unwrap();
        let index_buffer = GpuBuffer::create_device_local_with_data(
            context, allocator, "Vegetation LOD Indices", vk::BufferUsageFlags::INDEX_BUFFER, &indices,
        )?;
        let instance_buffer = if instances.is_empty() { None } else {
            Some(GpuBuffer::create_device_local_with_data(
                context, allocator, "Vegetation LOD Instances", vk::BufferUsageFlags::VERTEX_BUFFER, &instances,
            )?)
        };
        println!("[Vegetation] {name} LOD{lod}: {} triangles, {} instances", indices.len() / 3, instances.len());
        lods.push(VegetationLodGpu {
            index_buffer,
            index_count: indices.len() as u32,
            instance_buffer,
            instance_count: instances.len() as u32,
        });
    }
    Ok(VegetationGpuMesh { vertex_buffer, lods: lods.try_into().ok().unwrap() })
}

pub struct VulkanRenderer {
    pub context: VulkanContext,
    pub allocator: GpuAllocator,
    pub swapchain: VulkanSwapchain,
    pub texture_array: VulkanTextureArray,
    pub terrain_pipeline: VulkanPipeline,
    pub vegetation_pipeline: VegetationPipeline,
    pub sky_pipeline: SkyPipeline,
    pub water_pipeline: WaterPipeline,
    pub uniform_buffers: Vec<GpuBuffer>,

    // Maillage du Terrain
    pub terrain_vertex_buffer: GpuBuffer,
    pub terrain_index_buffer: GpuBuffer,
    pub terrain_index_count: u32,

    // Ciel atmosphérique
    pub sky_vertex_buffer: GpuBuffer,
    pub sky_index_buffer: GpuBuffer,
    pub sky_index_count: u32,

    // Océan / Eau animée
    pub water_vertex_buffer: GpuBuffer,
    pub water_index_buffer: GpuBuffer,
    pub water_index_count: u32,

    pine_vegetation: VegetationGpuMesh,
    broadleaf_vegetation: VegetationGpuMesh,
    bush_vegetation: VegetationGpuMesh,

    pub command_buffers: Vec<vk::CommandBuffer>,
    pub sync_objects: SyncObjects,
    pub current_frame: usize,
    pub resized: bool,
}

impl VulkanRenderer {
    pub fn new(window: &Window, world: &TerrainWorld) -> Result<Self, Box<dyn std::error::Error>> {
        let context = VulkanContext::new(window)?;
        let allocator = GpuAllocator::new(&context.instance, &context.device, context.physical_device)?;
        let swapchain = VulkanSwapchain::new(&context, &allocator, window)?;

        // 1. Textures PBR réelles (Sable, Herbe, Roche, Neige)
        println!("[Moteur] Chargement des textures PBR 1K...");
        let texture_array = VulkanTextureArray::load_terrain_textures(&context, &allocator)?;

        // 2. Uniform Buffers pour chaque trame en vol
        let uniform_size = std::mem::size_of::<CameraUniform>() as vk::DeviceSize;
        let mut uniform_buffers = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            let ubo = GpuBuffer::create_uniform_buffer(&context, &allocator, "Camera UBO", uniform_size)?;
            uniform_buffers.push(ubo);
        }

        // 3. Pipelines graphiques optimisées
        println!("[Moteur] Compilation des pipelines graphiques Vulkan...");
        let terrain_pipeline = VulkanPipeline::new(&context, swapchain.render_pass, &uniform_buffers, &texture_array)?;
        let vegetation_pipeline = VegetationPipeline::new(&context, swapchain.render_pass, &uniform_buffers, &texture_array)?;
        let sky_pipeline = SkyPipeline::new(&context, swapchain.render_pass, &uniform_buffers)?;
        let water_pipeline = WaterPipeline::new(&context, swapchain.render_pass, &uniform_buffers)?;

        // 4. Génération du terrain procédural haute définition
        println!("[Terrain] Generation du maillage continu (448m x 448m)...");
        let (vertices, indices) = world.build_world_mesh(0.0, 0.0);
        let terrain_index_count = indices.len() as u32;
        println!(
            "[Terrain] Maillage genere avec succes: {} sommets, {} triangles",
            vertices.len(),
            indices.len() / 3
        );

        let terrain_vertex_buffer = GpuBuffer::create_device_local_with_data(
            &context,
            &allocator,
            "Terrain Vertex Buffer",
            vk::BufferUsageFlags::VERTEX_BUFFER,
            &vertices,
        )?;
        let terrain_index_buffer = GpuBuffer::create_device_local_with_data(
            &context,
            &allocator,
            "Terrain Index Buffer",
            vk::BufferUsageFlags::INDEX_BUFFER,
            &indices,
        )?;

        // 5. Génération du Dôme de ciel atmosphérique
        println!("[Environnement] Generation du dome atmospherique...");
        let sky_mesh = SkyMesh::create_sky_dome();
        let sky_index_count = sky_mesh.indices.len() as u32;
        let sky_vertex_buffer = GpuBuffer::create_device_local_with_data(
            &context,
            &allocator,
            "Sky Vertex Buffer",
            vk::BufferUsageFlags::VERTEX_BUFFER,
            &sky_mesh.vertices,
        )?;
        let sky_index_buffer = GpuBuffer::create_device_local_with_data(
            &context,
            &allocator,
            "Sky Index Buffer",
            vk::BufferUsageFlags::INDEX_BUFFER,
            &sky_mesh.indices,
        )?;

        // 6. Génération de la surface de l'Océan
        println!("[Environnement] Generation du plan d'eau anime...");
        let water_mesh = WaterMesh::create_ocean_plane(0.0, 0.0, 520.0, 80, 1.0);
        let water_index_count = water_mesh.indices.len() as u32;
        let water_vertex_buffer = GpuBuffer::create_device_local_with_data(
            &context,
            &allocator,
            "Water Vertex Buffer",
            vk::BufferUsageFlags::VERTEX_BUFFER,
            &water_mesh.vertices,
        )?;
        let water_index_buffer = GpuBuffer::create_device_local_with_data(
            &context,
            &allocator,
            "Water Index Buffer",
            vk::BufferUsageFlags::INDEX_BUFFER,
            &water_mesh.indices,
        )?;

        // 7. Implantation procédurale et modèles Quaternius CC0 avec LOD
        println!("[Vegetation] Implantation procedurale de la vegetation sur le terrain...");
        let spawned = VegetationSpawner::spawn_for_world(&world.generator, 0.0, 0.0, 220.0, 1337);
        let total_veg = spawned.pines.len() + spawned.broadleafs.len() + spawned.bushes.len();
        println!(
            "[Vegetation] Implantation terminee : {} sapins, {} arbres feuillus, {} buissons (Total: {} instances VRAM)",
            spawned.pines.len(),
            spawned.broadleafs.len(),
            spawned.bushes.len(),
            total_veg
        );

        let asset_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/models/vegetation/quaternius/glTF");
        let pine_vegetation = upload_vegetation(
            &context, &allocator, "Quaternius Pine", load_quaternius_lods(&asset_dir.join("Pine_1.gltf"), 10.5)?,
            &spawned.pines, [55.0, 130.0],
        )?;
        let broadleaf_vegetation = upload_vegetation(
            &context, &allocator, "Quaternius Broadleaf", load_quaternius_lods(&asset_dir.join("CommonTree_1.gltf"), 8.0)?,
            &spawned.broadleafs, [50.0, 120.0],
        )?;
        let bush_vegetation = upload_vegetation(
            &context, &allocator, "Quaternius Bush", load_quaternius_lods(&asset_dir.join("Bush_Common.gltf"), 1.8)?,
            &spawned.bushes, [30.0, 75.0],
        )?;

        // 9. Command Buffers et Synchronisation
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(context.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(MAX_FRAMES_IN_FLIGHT as u32);
        let command_buffers = unsafe { context.device.allocate_command_buffers(&alloc_info)? };
        let sync_objects = SyncObjects::new(&context, MAX_FRAMES_IN_FLIGHT)?;

        Ok(Self {
            context,
            allocator,
            swapchain,
            texture_array,
            terrain_pipeline,
            vegetation_pipeline,
            sky_pipeline,
            water_pipeline,
            uniform_buffers,
            terrain_vertex_buffer,
            terrain_index_buffer,
            terrain_index_count,
            sky_vertex_buffer,
            sky_index_buffer,
            sky_index_count,
            water_vertex_buffer,
            water_index_buffer,
            water_index_count,
            pine_vegetation,
            broadleaf_vegetation,
            bush_vegetation,
            command_buffers,
            sync_objects,
            current_frame: 0,
            resized: false,
        })
    }

    pub fn recreate_swapchain(&mut self, window: &Window) -> Result<(), Box<dyn std::error::Error>> {
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }

        unsafe {
            self.context.device.device_wait_idle()?;
        }

        self.swapchain.destroy(&self.context, &self.allocator);
        self.swapchain = VulkanSwapchain::new(&self.context, &self.allocator, window)?;
        self.resized = false;
        Ok(())
    }

    pub fn render(
        &mut self,
        window: &Window,
        camera_uniform: &CameraUniform,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let frame = self.current_frame;

        unsafe {
            // Attente de la trame précédente
            self.context.device.wait_for_fences(
                &[self.sync_objects.in_flight_fences[frame]],
                true,
                u64::MAX,
            )?;

            // Acquisition de l'image de swapchain
            let image_index = match self.swapchain.swapchain_loader.acquire_next_image(
                self.swapchain.swapchain,
                u64::MAX,
                self.sync_objects.image_available_semaphores[frame],
                vk::Fence::null(),
            ) {
                Ok((index, _)) => index,
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) | Err(vk::Result::SUBOPTIMAL_KHR) => {
                    self.recreate_swapchain(window)?;
                    return Ok(());
                }
                Err(e) => return Err(e.into()),
            };

            self.context.device.reset_fences(&[self.sync_objects.in_flight_fences[frame]])?;

            // Mise à jour de l'Uniform Buffer caméra
            self.uniform_buffers[frame].update_data(camera_uniform)?;

            // Enregistrement des commandes graphiques
            let cmd = self.command_buffers[frame];
            self.context.device.reset_command_buffer(cmd, vk::CommandBufferResetFlags::empty())?;

            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.context.device.begin_command_buffer(cmd, &begin_info)?;

            // Fond atmosphérique et tampon de profondeur
            let clear_values = [
                vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.68, 0.78, 0.90, 1.0],
                    },
                },
                vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                },
            ];

            let render_pass_begin = vk::RenderPassBeginInfo::default()
                .render_pass(self.swapchain.render_pass)
                .framebuffer(self.swapchain.framebuffers[image_index as usize])
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.swapchain.extent,
                })
                .clear_values(&clear_values);

            self.context.device.cmd_begin_render_pass(
                cmd,
                &render_pass_begin,
                vk::SubpassContents::INLINE,
            );

            // Viewport et Scissor dynamiques
            let viewport = [vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: self.swapchain.extent.width as f32,
                height: self.swapchain.extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            }];
            self.context.device.cmd_set_viewport(cmd, 0, &viewport);

            let scissor = [vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.swapchain.extent,
            }];
            self.context.device.cmd_set_scissor(cmd, 0, &scissor);

            // -----------------------------------------------------------------
            // PASSE 1 : Dôme de ciel atmosphérique avec diffusion Rayleigh & Mie
            // -----------------------------------------------------------------
            self.context.device.cmd_bind_pipeline(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.sky_pipeline.pipeline,
            );
            self.context.device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.sky_pipeline.pipeline_layout,
                0,
                &[self.sky_pipeline.descriptor_sets[frame]],
                &[],
            );
            self.context.device.cmd_bind_vertex_buffers(
                cmd,
                0,
                &[self.sky_vertex_buffer.buffer],
                &[0],
            );
            self.context.device.cmd_bind_index_buffer(
                cmd,
                self.sky_index_buffer.buffer,
                0,
                vk::IndexType::UINT32,
            );
            self.context.device.cmd_draw_indexed(cmd, self.sky_index_count, 1, 0, 0, 0);

            // -----------------------------------------------------------------
            // PASSE 2 : Terrain procédural haute définition avec textures PBR 1K
            // -----------------------------------------------------------------
            self.context.device.cmd_bind_pipeline(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.terrain_pipeline.pipeline,
            );
            self.context.device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.terrain_pipeline.pipeline_layout,
                0,
                &[self.terrain_pipeline.descriptor_sets[frame]],
                &[],
            );
            self.context.device.cmd_bind_vertex_buffers(
                cmd,
                0,
                &[self.terrain_vertex_buffer.buffer],
                &[0],
            );
            self.context.device.cmd_bind_index_buffer(
                cmd,
                self.terrain_index_buffer.buffer,
                0,
                vk::IndexType::UINT32,
            );
            self.context.device.cmd_draw_indexed(cmd, self.terrain_index_count, 1, 0, 0, 0);

            // -----------------------------------------------------------------
            // PASSE 3 : Végétation 3D instanciée avec balancement dynamique au vent
            // -----------------------------------------------------------------
            self.context.device.cmd_bind_pipeline(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.vegetation_pipeline.pipeline,
            );
            self.context.device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.vegetation_pipeline.pipeline_layout,
                0,
                &[self.vegetation_pipeline.descriptor_sets[frame]],
                &[],
            );

            self.pine_vegetation.draw(&self.context, cmd);
            self.broadleaf_vegetation.draw(&self.context, cmd);
            self.bush_vegetation.draw(&self.context, cmd);

            // -----------------------------------------------------------------
            // PASSE 4 : Océan / Plan d'eau animé (reflets Fresnel, écume, vagues)
            // -----------------------------------------------------------------
            self.context.device.cmd_bind_pipeline(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.water_pipeline.pipeline,
            );
            self.context.device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.water_pipeline.pipeline_layout,
                0,
                &[self.water_pipeline.descriptor_sets[frame]],
                &[],
            );
            self.context.device.cmd_bind_vertex_buffers(
                cmd,
                0,
                &[self.water_vertex_buffer.buffer],
                &[0],
            );
            self.context.device.cmd_bind_index_buffer(
                cmd,
                self.water_index_buffer.buffer,
                0,
                vk::IndexType::UINT32,
            );
            self.context.device.cmd_draw_indexed(cmd, self.water_index_count, 1, 0, 0, 0);

            self.context.device.cmd_end_render_pass(cmd);
            self.context.device.end_command_buffer(cmd)?;

            // Soumission de la trame
            let wait_semaphores = [self.sync_objects.image_available_semaphores[frame]];
            let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let signal_semaphores = [self.sync_objects.render_finished_semaphores[frame]];
            let command_buffers_to_submit = [cmd];

            let submit_info = [vk::SubmitInfo::default()
                .wait_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&wait_stages)
                .command_buffers(&command_buffers_to_submit)
                .signal_semaphores(&signal_semaphores)];

            self.context.device.queue_submit(
                self.context.graphics_queue,
                &submit_info,
                self.sync_objects.in_flight_fences[frame],
            )?;

            // Présentation à l'écran
            let swapchains = [self.swapchain.swapchain];
            let image_indices = [image_index];
            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(&signal_semaphores)
                .swapchains(&swapchains)
                .image_indices(&image_indices);

            let present_result = self.swapchain.swapchain_loader.queue_present(
                self.context.present_queue,
                &present_info,
            );

            if self.resized
                || present_result == Err(vk::Result::ERROR_OUT_OF_DATE_KHR)
                || present_result == Err(vk::Result::SUBOPTIMAL_KHR)
            {
                self.recreate_swapchain(window)?;
            } else if let Err(e) = present_result {
                return Err(e.into());
            }

            self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
        }

        Ok(())
    }

    pub fn destroy(&mut self) {
        unsafe {
            self.context.device.device_wait_idle().ok();
        }
        self.sync_objects.destroy(&self.context);
        self.terrain_pipeline.destroy(&self.context);
        self.vegetation_pipeline.destroy(&self.context);
        self.sky_pipeline.destroy(&self.context);
        self.water_pipeline.destroy(&self.context);
        self.texture_array.destroy(&self.context, &self.allocator);
        for mut ubo in self.uniform_buffers.drain(..) {
            ubo.destroy(&self.context, &self.allocator);
        }
        self.terrain_vertex_buffer.destroy(&self.context, &self.allocator);
        self.terrain_index_buffer.destroy(&self.context, &self.allocator);
        self.sky_vertex_buffer.destroy(&self.context, &self.allocator);
        self.sky_index_buffer.destroy(&self.context, &self.allocator);
        self.water_vertex_buffer.destroy(&self.context, &self.allocator);
        self.water_index_buffer.destroy(&self.context, &self.allocator);
        self.pine_vegetation.destroy(&self.context, &self.allocator);
        self.broadleaf_vegetation.destroy(&self.context, &self.allocator);
        self.bush_vegetation.destroy(&self.context, &self.allocator);
        self.swapchain.destroy(&self.context, &self.allocator);
    }
}

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        self.destroy();
    }
}

pub struct App {
    window: Option<Window>,
    renderer: Option<VulkanRenderer>,
    terrain_world: TerrainWorld,
    camera: Camera,
    input: InputState,
    last_time: Instant,
    start_time: Instant,
    fps_timer: Instant,
    frame_count: u32,
}

impl App {
    pub fn new() -> Self {
        let terrain_world = TerrainWorld::new(1337);
        let start_height = terrain_world.sample_height(0.0, 0.0);
        let camera = Camera::new(Vec3::new(0.0, start_height + 2.0, 0.0));

        Self {
            window: None,
            renderer: None,
            terrain_world,
            camera,
            input: InputState::new(),
            last_time: Instant::now(),
            start_time: Instant::now(),
            fps_timer: Instant::now(),
            frame_count: 0,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = WindowAttributes::default()
            .with_title("Vulkan Procedural World 3D (Rust + Ash)")
            .with_inner_size(PhysicalSize::new(1600, 900));

        let window = match event_loop.create_window(attributes) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("Erreur lors de la creation de la fenetre winit: {:?}", e);
                event_loop.exit();
                return;
            }
        };

        // Capture du curseur pour vue FPS
        window.set_cursor_visible(false);
        let _ = window.set_cursor_grab(CursorGrabMode::Confined)
            .or_else(|_| window.set_cursor_grab(CursorGrabMode::Locked));

        let renderer = match VulkanRenderer::new(&window, &self.terrain_world) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Erreur critique lors de l'initialisation Vulkan: {:?}", e);
                event_loop.exit();
                return;
            }
        };

        self.window = Some(window);
        self.renderer = Some(renderer);
        self.last_time = Instant::now();
        self.start_time = Instant::now();
        self.fps_timer = Instant::now();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(_) => {
                if let Some(ref mut renderer) = self.renderer {
                    renderer.resized = true;
                }
            }
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    physical_key: PhysicalKey::Code(key),
                    state,
                    ..
                },
                ..
            } => {
                let pressed = state == ElementState::Pressed;
                if key == KeyCode::Escape && pressed {
                    // Touche Echap: alterner capture de la souris ou quitter
                    if let Some(ref window) = self.window {
                        self.input.cursor_locked = !self.input.cursor_locked;
                        window.set_cursor_visible(!self.input.cursor_locked);
                        let _ = if self.input.cursor_locked {
                            window.set_cursor_grab(CursorGrabMode::Confined)
                                .or_else(|_| window.set_cursor_grab(CursorGrabMode::Locked))
                        } else {
                            window.set_cursor_grab(CursorGrabMode::None)
                        };
                    }
                } else {
                    self.input.on_key_event(key, pressed);
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last_time).as_secs_f32().min(0.1);
                self.last_time = now;

                // Mise à jour de la physique et de la caméra
                let world_ref = &self.terrain_world;
                self.camera.update(dt, &mut self.input, |x, z| world_ref.sample_height(x, z));

                // Compteur FPS dans le titre de fenêtre
                self.frame_count += 1;
                if self.fps_timer.elapsed().as_secs_f32() >= 1.0 {
                    let fps = self.frame_count as f32 / self.fps_timer.elapsed().as_secs_f32();
                    let mode_str = if self.camera.is_freecam { "Flycam (Vol)" } else { "Marcheur FPS" };
                    let mouse_y_str = if self.camera.invert_mouse_y { "Inverse" } else { "Normal" };
                    if let Some(ref window) = self.window {
                        window.set_title(&format!(
                            "Vulkan 3D World [Vegetation & Ocean PBR] | FPS: {:.0} | Mode: {} (F) | Axe Y: {} (Touche 'I') | Pos: ({:.0}, {:.0}, {:.0})",
                            fps, mode_str, mouse_y_str, self.camera.position.x, self.camera.position.y, self.camera.position.z
                        ));
                    }
                    self.frame_count = 0;
                    self.fps_timer = Instant::now();
                }

                // Rendu
                if let (Some(ref window), Some(ref mut renderer)) = (&self.window, &mut self.renderer) {
                    let aspect = renderer.swapchain.extent.width as f32 / renderer.swapchain.extent.height as f32;
                    let elapsed_secs = self.start_time.elapsed().as_secs_f32();
                    let uniform = self.camera.build_uniform(aspect, elapsed_secs);

                    if let Err(e) = renderer.render(window, &uniform) {
                        eprintln!("Erreur lors du rendu Vulkan: {:?}", e);
                    }
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _id: DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            self.input.on_mouse_move(delta.0, delta.1);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(ref window) = self.window {
            window.request_redraw();
        }
    }
}

fn main() {
    println!("============================================================");
    println!("     VULKAN PROCEDURAL 3D TERRAIN ENGINE (RUST + ASH)       ");
    println!("============================================================");
    println!("Controles :");
    println!("  - Deplacement : ZQSD ou WASD");
    println!("  - Regard : Souris");
    println!("  - Saut : Espace");
    println!("  - Sprint : Maj (Shift)");
    println!("  - Accroupi / Descendre en vol : Ctrl ou C");
    println!("  - Bascule Mode Vol / Marcheur : F");
    println!("  - Inverser axe vertical souris : I");
    println!("  - Verrouillage / Liberation curseur : Echap");
    println!("============================================================");

    let event_loop = EventLoop::new().expect("Echec d'initialisation de l'EventLoop winit");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new();
    event_loop.run_app(&mut app).expect("Echec de boucle d'evenement");
}
