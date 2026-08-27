use glam::{Mat4, Quat, Vec3};

use wgpu::{util::DeviceExt};

use crate::{bg_index, state::GlobalState};

const NUM_INSTANCES_PER_ROW: u32 = 10;
const INSTANCE_DISPLACEMENT: Vec3 = Vec3::new(
    NUM_INSTANCES_PER_ROW as f32 * 0.5,
    0.0,
    NUM_INSTANCES_PER_ROW as f32 * 0.5,
);

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    model: [[f32; 4]; 4],
}

pub struct Instance {
    position: Vec3,
    rotation: Quat,
}

impl Instance {
    fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (Mat4::from_translation(self.position) * Mat4::from_quat(self.rotation))
                .to_cols_array_2d(),
        }
    }
}

pub struct InstanceContext {
    pub instances: Vec<Instance>,
    pub(super) buffer: wgpu::Buffer,
    pub(crate) bg: wgpu::BindGroup,
}
impl InstanceContext {
    pub fn new_default(
        glb: &GlobalState,
        instances_bg_layout: &wgpu::BindGroupLayout,
    ) -> anyhow::Result<Self> {
        const SPACE_BETWEEN: f32 = 3.0;
        let instances = (0..NUM_INSTANCES_PER_ROW)
            .flat_map(|z| {
                (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                    let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                    let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);

                    let position = Vec3 {
                        x: x as f32,
                        y: 0.0,
                        z: z as f32,
                    } - INSTANCE_DISPLACEMENT;

                    let rotation = if position == Vec3::ZERO {
                        // this is needed so an object at (0, 0, 0) won't get scaled to zero
                        // as Quaternions can affect scale if they're not created correctly
                        Quat::from_axis_angle(Vec3::Z, 0f32.to_radians())
                    } else {
                        Quat::from_axis_angle(position.normalize(), 45f32.to_radians())
                    };

                    Instance { position, rotation }
                })
            })
            .collect::<Vec<_>>();
        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let buffer = glb
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
            });
        let bg = Self::create_bind_group(glb, &buffer, instances_bg_layout);
        Ok(Self {
            instances,
            buffer,
            bg,
        })
    }

    pub fn new_one(
        glb: &GlobalState,
        instances_bg_layout: &wgpu::BindGroupLayout,
    ) -> anyhow::Result<Self> {
        let instance = Instance {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        };

        let instance_data = vec![instance.to_raw()];
        let instances = vec![instance];
        let buffer = glb
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
            });

        let bg = Self::create_bind_group(glb, &buffer, instances_bg_layout);
        Ok(Self {
            instances,
            buffer,
            bg,
        })
    }
    pub fn new(
        glb: &GlobalState,
        instances_bg_layout: &wgpu::BindGroupLayout,
        instances: Vec<Instance>,
    ) -> anyhow::Result<Self> {
        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let buffer = glb
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
            });
        let bg = Self::create_bind_group(glb, &buffer, instances_bg_layout);
        Ok(Self {
            instances,
            buffer,
            bg,
        })
    }

    /// Creates the BindGroupLayout and BindGroup for the instance buffer.
    pub fn create_bind_group_layout(glb: &GlobalState) -> wgpu::BindGroupLayout {
        let layout = glb
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Instance Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        layout
    }

    /// Creates the BindGroupLayout and BindGroup for the instance buffer.
    fn create_bind_group(
        glb: &GlobalState,
        buffer: &wgpu::Buffer,
        bg_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        let bind_group = glb.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Instance Bind Group"),
            layout: &bg_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        bind_group
    }

    pub fn bind(&self, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_bind_group(bg_index::INSTANCES, &self.bg, &[]);
    }
}
