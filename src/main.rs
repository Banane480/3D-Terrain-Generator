mod camera;
mod input;
mod terrain;
mod vulkan;

use std::time::Instant;
use ash::vk;
use glam::Vec3;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, DeviceId, ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowAttributes, WindowId};

use crate::camera::{Camera, CameraUniform};
use crate::input::InputState;
use crate::terrain::TerrainWorld;
use crate::vulkan::{
    GpuAllocator, GpuBuffer, SyncObjects, VulkanContext, VulkanPipeline, VulkanSwapchain,
    VulkanTextureArray,
};

const MAX_FRAMES_IN_FLIGHT: usize = 2;

pub struct VulkanRenderer {
    pub context: VulkanContext,
    pub allocator: GpuAllocator,
    pub swapchain: VulkanSwapchain,
    pub texture_array: VulkanTextureArray,
    pub pipeline: VulkanPipeline,
    pub uniform_buffers: Vec<GpuBuffer>,
    pub vertex_buffer: GpuBuffer,
    pub index_buffer: GpuBuffer,
    pub index_count: u32,
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

        // 1. Chargement des textures PBR réelles (Sable, Herbe, Roche, Neige)
        println!("[Moteur] Chargement des textures PBR haute definition...");
        let texture_array = VulkanTextureArray::load_terrain_textures(&context, &allocator)?;

        // 2. Uniform Buffers pour chaque frame en vol
        let uniform_size = std::mem::size_of::<CameraUniform>() as vk::DeviceSize;
        let mut uniform_buffers = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            let ubo = GpuBuffer::create_uniform_buffer(&context, &allocator, "Camera UBO", uniform_size)?;
            uniform_buffers.push(ubo);
        }

        // 3. Pipeline graphique avec Shaders GLSL et Textures PBR
        let pipeline = VulkanPipeline::new(&context, swapchain.render_pass, &uniform_buffers, &texture_array)?;

        // 4. Génération du terrain procédural continu haute définition
        println!("[Terrain] Generation du maillage continu autour du joueur...");
        let (vertices, indices) = world.build_world_mesh(0.0, 0.0);
        let index_count = indices.len() as u32;
        println!(
            "[Terrain] Maillage genere avec succes: {} sommets, {} triangles",
            vertices.len(),
            indices.len() / 3
        );

        // 5. Transfert VRAM ultra-rapide (GpuOnly) via Staging Buffer
        let vertex_buffer = GpuBuffer::create_device_local_with_data(
            &context,
            &allocator,
            "Terrain Vertex Buffer",
            vk::BufferUsageFlags::VERTEX_BUFFER,
            &vertices,
        )?;

        let index_buffer = GpuBuffer::create_device_local_with_data(
            &context,
            &allocator,
            "Terrain Index Buffer",
            vk::BufferUsageFlags::INDEX_BUFFER,
            &indices,
        )?;

        // 6. Allocation des Command Buffers
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(context.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(MAX_FRAMES_IN_FLIGHT as u32);
        let command_buffers = unsafe { context.device.allocate_command_buffers(&alloc_info)? };

        // 7. Synchronisation (Semaphores et Fences)
        let sync_objects = SyncObjects::new(&context, MAX_FRAMES_IN_FLIGHT)?;

        Ok(Self {
            context,
            allocator,
            swapchain,
            texture_array,
            pipeline,
            uniform_buffers,
            vertex_buffer,
            index_buffer,
            index_count,
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
            // Attente de la trame precedente
            self.context.device.wait_for_fences(
                &[self.sync_objects.in_flight_fences[frame]],
                true,
                u64::MAX,
            )?;

            // Acquisition de l'image suivante
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

            // Mise a jour de l'Uniform Buffer camera
            self.uniform_buffers[frame].update_data(camera_uniform)?;

            // Enregistrement des commandes
            let cmd = self.command_buffers[frame];
            self.context.device.reset_command_buffer(cmd, vk::CommandBufferResetFlags::empty())?;

            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.context.device.begin_command_buffer(cmd, &begin_info)?;

            // Ciel atmospherique en arriere-plan
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

            self.context.device.cmd_bind_pipeline(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline.pipeline,
            );

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

            self.context.device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline.pipeline_layout,
                0,
                &[self.pipeline.descriptor_sets[frame]],
                &[],
            );

            self.context.device.cmd_bind_vertex_buffers(
                cmd,
                0,
                &[self.vertex_buffer.buffer],
                &[0],
            );
            self.context.device.cmd_bind_index_buffer(
                cmd,
                self.index_buffer.buffer,
                0,
                vk::IndexType::UINT32,
            );

            // Rendu indexe du terrain
            self.context.device.cmd_draw_indexed(cmd, self.index_count, 1, 0, 0, 0);

            self.context.device.cmd_end_render_pass(cmd);
            self.context.device.end_command_buffer(cmd)?;

            // Soumission
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

            // Presentation
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
        self.pipeline.destroy(&self.context);
        self.texture_array.destroy(&self.context, &self.allocator);
        for mut ubo in self.uniform_buffers.drain(..) {
            ubo.destroy(&self.context, &self.allocator);
        }
        self.vertex_buffer.destroy(&self.context, &self.allocator);
        self.index_buffer.destroy(&self.context, &self.allocator);
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

                // Mise a jour de la physique et de la camera
                let world_ref = &self.terrain_world;
                self.camera.update(dt, &mut self.input, |x, z| world_ref.sample_height(x, z));

                // Compteur FPS dans le titre de fenetre
                self.frame_count += 1;
                if self.fps_timer.elapsed().as_secs_f32() >= 1.0 {
                    let fps = self.frame_count as f32 / self.fps_timer.elapsed().as_secs_f32();
                    let mode_str = if self.camera.is_freecam { "Flycam (Vol)" } else { "Marcheur FPS" };
                    let mouse_y_str = if self.camera.invert_mouse_y { "Inversé" } else { "Normal" };
                    if let Some(ref window) = self.window {
                        window.set_title(&format!(
                            "Vulkan 3D World [PBR Textures HD] | FPS: {:.0} | Mode: {} (F) | Axe Y: {} (Touche 'I') | Pos: ({:.0}, {:.0}, {:.0})",
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
