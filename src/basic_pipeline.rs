use crate::{
    buffer::Vertex, camera::CameraContext, load_shader_str, state::GlobalState, texture::TextureContext,
};

pub struct BasicPipeline {
    render_pipeline: wgpu::RenderPipeline,
}

impl BasicPipeline {
    pub fn new(
        glb: &GlobalState,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> anyhow::Result<Self> {

        let shader = glb
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("basic_pipeline.wgsl"),
                source: wgpu::ShaderSource::Wgsl(load_shader_str!("basic_pipeline.wgsl").into()),
            });

        let render_pipeline_layout =
            glb.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Basic Render Pipeline Layout"),
                    bind_group_layouts: &[
                        Some(&texture_bind_group_layout),
                        Some(&camera_bind_group_layout),
                    ],
                    immediate_size: 0,
                });

        let render_pipeline = glb
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Basic Render Pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Some(Vertex::desc())],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    // 3.
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        // 4.
                        format: glb.config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList, // 1.
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw, // 2.
                    cull_mode: Some(wgpu::Face::Back),
                    // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                    polygon_mode: wgpu::PolygonMode::Fill,
                    // Requires Features::DEPTH_CLIP_CONTROL
                    unclipped_depth: false,
                    // Requires Features::CONSERVATIVE_RASTERIZATION
                    conservative: false,
                },
                depth_stencil: None, // 1.
                multisample: wgpu::MultisampleState {
                    count: 1,                         // 2.
                    mask: !0,                         // 3.
                    alpha_to_coverage_enabled: false, // 4.
                },
                multiview_mask: None, // 5.
                cache: None,          // 6.
            });

        Ok(Self { render_pipeline })
    }

    pub fn set(&self, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_pipeline(&self.render_pipeline);
    }
    pub fn bind_texture(&self, render_pass: &mut wgpu::RenderPass, texture_c: &TextureContext) {
        texture_c.bind(0, render_pass);
    }
    pub fn bind_camera(&self, render_pass: &mut wgpu::RenderPass, camera_c: &CameraContext) {
        camera_c.bind(1, render_pass);
    }
}
