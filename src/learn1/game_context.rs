use std::time::{SystemTime, UNIX_EPOCH};

use glam::Vec4;
use wgpu::{Color, util::DeviceExt};
use winit::{dpi::PhysicalPosition, event_loop::ActiveEventLoop};

use amar_engine::{
    input_handler::InputHandler,
    registry::{registry_view::PluginRegistryView, static_ptr::StaticRef},
    state::GlobalState,
};
pub fn color_to_vec4(color: wgpu::Color) -> glam::Vec4 {
    glam::Vec4::new(
        color.r as f32,
        color.g as f32,
        color.b as f32,
        color.a as f32,
    )
}
pub fn vec4_to_color(color: glam::Vec4) -> wgpu::Color {
    wgpu::Color {
        r: color.x as f64,
        g: color.y as f64,
        b: color.z as f64,
        a: color.w as f64,
    }
}

pub struct GameContext {
    loop_timer_buffer: wgpu::Buffer,
    global_color_buffer: wgpu::Buffer,
    pub bg_layout: wgpu::BindGroupLayout,
    bg: wgpu::BindGroup,
    mouse_move_color: wgpu::Color,
}

impl GameContext {
    pub fn new(glb: &GlobalState) -> anyhow::Result<Self> {
        let loop_timer_buffer = glb
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Loop Time Buffer"),
                contents: bytemuck::cast_slice(&[get_loop_timer()]),
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
                        visibility: wgpu::ShaderStages::FRAGMENT,
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
            mouse_move_color: Color::WHITE,
        })
    }
    pub fn bind(&self, index: u32, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_bind_group(index, &self.bg, &[]);
    }
}

fn get_loop_timer() -> f32 {
    let mut time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
        % 1000;
    if time > 500 {
        time = 1000 - time
    }
    time as f32 / 500 as f32
}

pub fn sys_update_game(
    (game_ctx, glb, event_loop, input_handler): &mut (
        GameContext,
        GlobalState,
        StaticRef<ActiveEventLoop>,
        InputHandler,
    ),
    _: PluginRegistryView,
) {
    let loop_timer_color = Vec4::new(get_loop_timer(), 1f32 - get_loop_timer(), 0.6, 1.0);
    let final_color = (color_to_vec4(game_ctx.mouse_move_color) + loop_timer_color) / 2 as f32;
    glb.color = vec4_to_color(final_color);

    glb.queue.write_buffer(
        &game_ctx.loop_timer_buffer,
        0,
        bytemuck::cast_slice(&[get_loop_timer()]),
    );
    glb.queue.write_buffer(
        &game_ctx.global_color_buffer,
        0,
        bytemuck::cast_slice(&[color_to_vec4(glb.color)]),
    );

    let position: PhysicalPosition<f64> = input_handler.mouse_position();
    game_ctx.mouse_move_color = wgpu::Color {
        r: position.x / glb.config.width as f64,
        g: position.y / glb.config.width as f64,
        b: 0.5,
        a: 1.0,
    };
}
