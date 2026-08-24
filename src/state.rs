use crate::{
    camera::CameraContext,
    default_pipeline::DefaultPipeline,
    game_context::{GameContext, GameHandler},
    keys,
    mesh::{instanceb::InstanceContext, resource::load_model},
    registry::ResourceRegistry,
    texture::Texture,
};
use std::sync::Arc;
use wgpu::{Instance, InstanceDescriptor, RequestAdapterError};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::MouseButton,
    event_loop::ActiveEventLoop,
    keyboard::KeyCode,
    window::Window,
};

// This will store the state of our game
// lib.rs

pub struct GlobalState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    is_surface_configured: bool,
    pub config: wgpu::SurfaceConfiguration,
    pub color: wgpu::Color,
}
impl GlobalState {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = Instance::new(InstanceDescriptor::new_without_display_handle());

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = Self::get_adapter(&instance, &surface).await?;
        let (device, queue) = Self::get_device_queue(&adapter).await?;
        let config = Self::get_config(&surface, &adapter, &size);

        let color = wgpu::Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        };
        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            window,
            color,
        })
    }

    // region: helpers

    async fn get_adapter(
        instance: &wgpu::Instance,
        surface: &wgpu::Surface<'static>,
    ) -> Result<wgpu::Adapter, RequestAdapterError> {
        instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: true,
            })
            .await
    }

    async fn get_device_queue(
        adapter: &wgpu::Adapter,
    ) -> Result<(wgpu::Device, wgpu::Queue), wgpu::RequestDeviceError> {
        adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                // WebGL doesn't support all of wgpu's features, so if
                // we're building for the web we'll have to disable some.
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await
    }

    fn get_config(
        surface: &wgpu::Surface<'static>,
        adapter: &wgpu::Adapter,
        window_size: &PhysicalSize<u32>,
    ) -> wgpu::SurfaceConfiguration {
        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result in all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: window_size.width,
            height: window_size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            color_space: wgpu::SurfaceColorSpace::Auto,
        }
    }

    // endregion
}

/// Context is for custom or single instance objects
/// state is for single insstance structs
pub struct State {
    glb: GlobalState,
    registry: ResourceRegistry,
}
impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let glb = GlobalState::new(window).await?;
        let mut registry = ResourceRegistry::default();
        let diffuse_bg_layout = Self::get_diffuse_bg_layout(&glb);

        let camera_ctx = CameraContext::new(&glb)?;
        let game_ctx = GameContext::new(&glb)?;
        let instances_bg_layout = InstanceContext::create_bind_group_layout(&glb);

        let default_pipeline = DefaultPipeline::new(
            &glb,
            &diffuse_bg_layout,
            &camera_ctx.bg_layout,
            &game_ctx.bg_layout,
            &instances_bg_layout,
        )?;

        let depth_t = Texture::create_depth_texture(&glb, "depth_texture");

        registry.insert_res(camera_ctx);
        registry.insert_res(game_ctx);
        registry.insert(keys::DEFAULT, default_pipeline);
        registry.insert(keys::texture::DEPTH, depth_t);

        // initialise cube model and default,one instance_buffers
        registry.insert(
            keys::models::CUBE,
            load_model("cube/cube.obj", &glb.device, &glb.queue, &diffuse_bg_layout)?,
        );
        registry.insert(
            keys::DEFAULT,
            InstanceContext::new_default(&glb, &instances_bg_layout)?,
        );
        registry.insert(keys::ONE_INSTANCE, InstanceContext::new_one(&glb, &instances_bg_layout)?);

        registry.insert(keys::bg_layout::INSTANCES, instances_bg_layout);
        registry.insert(keys::bg_layout::DIFFUSE, diffuse_bg_layout);

        Ok(Self { glb, registry })
    }

    // region: helper functions

    fn get_diffuse_bg_layout(glb: &GlobalState) -> wgpu::BindGroupLayout {
        glb.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            })
    }
    // endregion

    // region: window & input handlers

    pub fn handle_resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.glb.config.width = width;
            self.glb.config.height = height;
            self.glb
                .surface
                .configure(&self.glb.device, &self.glb.config);
            self.glb.is_surface_configured = true;
        }

        // must recreate the depth_t with updates dimentions of framebuffer
        // if not it crashes on use at RenderPass creation by CommandEncoder.
        self.registry.insert(
            "depth",
            Texture::create_depth_texture(&self.glb, "depth_texture"),
        );
    }
    pub fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        match (code, is_pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            _ => {
                let camera_ctx = self.registry.get_res_mut::<CameraContext>().unwrap();
                camera_ctx.handle_key(code, is_pressed);

                self.registry
                    .with_res_mut::<(GameContext,), _>(|(game_ctx,), registry| {
                        game_ctx.handle_key(code, is_pressed, &mut self.glb, registry);
                    });
            }
        }
    }
    pub fn handle_mouse_key(
        &mut self,
        event_loop: &ActiveEventLoop,
        code: MouseButton,
        is_pressed: bool,
    ) {
        match (code, is_pressed) {
            _ => {
                self.registry
                    .with_res_mut::<(GameContext,), _>(|(game_ctx,), registry| {
                        game_ctx.handle_mouse_key(code, is_pressed, &mut self.glb, registry);
                    });
            }
        }
    }

    pub fn handle_mouse_moved(&mut self, position: PhysicalPosition<f64>) {
        self.registry
            .with_res_mut::<(GameContext,), _>(|(game_ctx,), registry| {
                game_ctx.handle_mouse_moved(position, &mut self.glb, registry);
            });
    }
    // endregion

    pub fn update(&mut self) {
        self.registry
            .with_res_mut::<(GameContext, CameraContext), _>(|(game_ctx, camera_ctx), _| {
                camera_ctx.update(&self.glb);
                game_ctx.update(&mut self.glb);
            });
    }
    pub fn render(&mut self) -> anyhow::Result<()> {
        self.glb.window.request_redraw();

        // We can't render unless the surface is configured
        if !self.glb.is_surface_configured {
            return Ok(());
        }

        let output = match self.glb.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => {
                // Skip this frame
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.glb
                    .surface
                    .configure(&self.glb.device, &self.glb.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                // You could recreate the devices and all resources
                // created with it here, but we'll just bail
                anyhow::bail!("Lost device");
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .glb
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let depth_t = self.registry.get::<Texture>("depth").unwrap();

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.glb.color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_t.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
            });

            self.registry
                .with_res_mut::<(GameContext,), _>(|(game_ctx,), registry| {
                    game_ctx.render(&mut render_pass, &self.glb, registry);
                });
        }

        self.glb.queue.submit(std::iter::once(encoder.finish()));
        self.glb.queue.present(output);

        Ok(())
    }
}
