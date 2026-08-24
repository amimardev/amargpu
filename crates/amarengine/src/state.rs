use crate::{
    input_handler::InputHandler,
    keys,
    mesh::instanceb::InstanceContext,
    plugins::{Plugin, PluginHandler},
    registry::{EventLoopRef, ResourceRegistry},
    texture::Texture,
};
use std::sync::Arc;
use wgpu::{Instance, InstanceDescriptor, RequestAdapterError};
use winit::{dpi::PhysicalSize, window::Window};

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
pub(crate) struct State {
    pub(super) registry: ResourceRegistry,
    pub(super) plugins: Vec<Box<dyn Plugin>>,
}
impl State {
    pub async fn new(window: Arc<Window>, plugins: Vec<Box<dyn Plugin>>) -> anyhow::Result<Self> {
        let glb = GlobalState::new(window).await?;
        let mut registry = ResourceRegistry::default();
        let diffuse_bg_layout = Texture::get_diffuse_bg_layout(&glb);
        let depth_t = Texture::create_depth_texture(&glb, "depth_texture");
        let instances_bg_layout = InstanceContext::create_bind_group_layout(&glb);

        registry.spawn(
            Some(keys::DEFAULT),
            InstanceContext::new_default(&glb, &instances_bg_layout)?,
        );
        registry.spawn(
            Some(keys::ONE_INSTANCE),
            InstanceContext::new_one(&glb, &instances_bg_layout)?,
        );

        registry.spawn(Some(keys::texture::DEPTH), depth_t);
        registry.spawn(Some(keys::bg_layout::INSTANCES), instances_bg_layout);
        registry.spawn(Some(keys::bg_layout::DIFFUSE), diffuse_bg_layout);

        registry.insert_res(InputHandler::default());
        registry.insert_res(glb);

        Ok(Self { registry, plugins })
    }

    // region: helper functions

    // endregion

    // region: window & input handlers
    fn glb(&self) -> &GlobalState {
        self.registry.get_res::<GlobalState>().unwrap()
    }
    fn glb_mut(&mut self) -> &mut GlobalState {
        self.registry.get_res_mut::<GlobalState>().unwrap()
    }

    pub fn handle_resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.registry
                .get_res_mut::<GlobalState>()
                .unwrap()
                .config
                .width = width;
            self.glb_mut().config.height = height;
            self.glb()
                .surface
                .configure(&self.glb().device, &self.glb().config);
            self.glb_mut().is_surface_configured = true;
        }

        // must recreate the depth_t with updates dimentions of framebuffer
        // if not it crashes on use at RenderPass creation by CommandEncoder.
        self.registry.spawn(
            Some(keys::texture::DEPTH),
            Texture::create_depth_texture(self.glb(), "depth_texture"),
        );
    }

    // endregion

    pub fn render(&mut self) -> anyhow::Result<()> {
        self.glb_mut().window.request_redraw();

        // We can't render unless the surface is configured
        if !self.glb_mut().is_surface_configured {
            return Ok(());
        }

        let output = match self.glb_mut().surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => {
                // Skip this frame
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.glb()
                    .surface
                    .configure(&self.glb().device, &self.glb().config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                // You could recreate the devices and all resources
                // created with it here, but we'll just bail
                anyhow::bail!("Lost device");
            }
        };

        let mut encoder =
            self.glb()
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        self.run_render_all(&output, &mut encoder);

        self.glb_mut()
            .queue
            .submit(std::iter::once(encoder.finish()));
        self.glb_mut().queue.present(output);

        Ok(())
    }
}
 