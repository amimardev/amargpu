use std::time::{SystemTime, UNIX_EPOCH};

use wgpu::util::DeviceExt; 

use crate::state::GlobalState;
fn color_to_vec4(color: wgpu::Color) -> glam::Vec4 {
    glam::Vec4::new(
        color.r as f32,
        color.g as f32,
        color.b as f32,
        color.a as f32,
    )
}
pub struct UniformVars {
    loop_timer_buffer: wgpu::Buffer,
    global_color_buffer: wgpu::Buffer,
    pub bg_layout: wgpu::BindGroupLayout,
    bg: wgpu::BindGroup,
}

impl UniformVars {
    pub fn new(glb: &GlobalState) -> anyhow::Result<Self> {
        let loop_timer_buffer = glb
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Loop Time Buffer"),
                contents: bytemuck::cast_slice(&[Self::get_loop_timer()]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let global_color_buffer =
            glb.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Global Color Buffer"),
                    contents: bytemuck::cast_slice(&[color_to_vec4(glb.color)]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let bg_layout = glb
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("uniform_vars_bind_group_layout"),
            });

        let bg = glb.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bg_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: loop_timer_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: global_color_buffer.as_entire_binding(),
                },
            ],
            label: Some("uniform_vars_bind_group"),
        });
 
        Ok(Self { 
            loop_timer_buffer,
            global_color_buffer,
            bg_layout,
            bg,
        })
    }
    pub fn bind(&self, index: u32, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_bind_group(index, &self.bg, &[]);
    }


    pub fn get_loop_timer() -> f32 {
        (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            % 64) as f32
            / 64 as f32
    }

    pub fn update(&mut self, glb: &GlobalState) {
        glb.queue.write_buffer(
            &self.loop_timer_buffer,
            0,
            bytemuck::cast_slice(&[Self::get_loop_timer()]),
        );
        glb.queue.write_buffer(
            &self.global_color_buffer,
            0,
            bytemuck::cast_slice(&[color_to_vec4(glb.color)]),
        );
    }
}
