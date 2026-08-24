use amarengine::{ 
    mesh::model::{ModelVertex, Vertex},
    state::GlobalState,
    texture::Texture,
};

use crate::{learn1::game_context::GameContext, load_shader_str, other_bg_index};

pub struct DefaultPipeline;

impl DefaultPipeline {
    pub fn new(
        glb: &GlobalState,
        texture_bg_layout: &wgpu::BindGroupLayout,
        camera_bg_layout: &wgpu::BindGroupLayout,
        uniform_vars_bg_layout: &wgpu::BindGroupLayout,
        instances_bg_layout: &wgpu::BindGroupLayout,
    ) -> anyhow::Result<wgpu::RenderPipeline> {
        let shader = glb
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("default_pipeline.wgsl"),
                source: wgpu::ShaderSource::Wgsl(load_shader_str!("default_pipeline.wgsl").into()),
            });

        let render_pipeline_layout =
            glb.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Basic Render Pipeline Layout"),
                    bind_group_layouts: &[
                        Some(&texture_bg_layout),
                        Some(&camera_bg_layout),
                        Some(&instances_bg_layout),
                        Some(&uniform_vars_bg_layout),
                    ],
                    immediate_size: 0,
                });

        let render_pipeline = glb
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Default Render Pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Some(ModelVertex::desc())],
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
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: Texture::DEPTH_FORMAT,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less), // 1.
                    stencil: wgpu::StencilState::default(),           // 2.
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,                         // 2.
                    mask: !0,                         // 3.
                    alpha_to_coverage_enabled: false, // 4.
                },
                multiview_mask: None, // 5.
                cache: None,          // 6.
            });

        Ok(render_pipeline)
    }

    pub fn bind_uniform_vars(render_pass: &mut wgpu::RenderPass, uniform_vars: &GameContext) {
        uniform_vars.bind(other_bg_index::GAME, render_pass);
    }
}
