use crate::texture::Texture;
use crate::{
    mesh::{instanceb::InstanceContext, resource::load_model},
    state::GlobalState,
};

pub trait Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
}
pub struct Material {
    pub name: String,
    pub diffuse_texture: Texture,
    pub bind_group: wgpu::BindGroup,
}

pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize,
}
pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

use std::collections::HashMap;

pub struct ModelStore {
    pub(super) models: HashMap<String, Model>,
    instance_c_map: HashMap<String, InstanceContext>,
}

impl ModelStore {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            instance_c_map: HashMap::new(),
        }
    }

    pub fn new_default(
        glb: &GlobalState,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> anyhow::Result<Self> {
        let obj_model = load_model(
            "cube/cube.obj",
            &glb.device,
            &glb.queue,
            &texture_bind_group_layout,
        )?;

        let instance_c_default = InstanceContext::new_default(glb)?;
        let instance_c_one = InstanceContext::new_one(glb)?;

        let mut instance_c_map = HashMap::new();

        let mut models_map = HashMap::new();

        models_map.insert("cube".to_string(), obj_model);

        instance_c_map.insert("default".into(), instance_c_default);
        instance_c_map.insert("one".into(), instance_c_one);

        Ok(Self {
            models: models_map,
            instance_c_map,
        })
    }
    fn set_instance_buffer(
        &self,
        render_pass: &mut wgpu::RenderPass,
        label: &str,
    ) -> anyhow::Result<u32> {
        let instance_c_option = self.instance_c_map.get(label);

        if let Some(i_c) = instance_c_option {
            render_pass.set_vertex_buffer(1, i_c.buffer.slice(..));
            return Ok(i_c.instances.len() as _);
        } else {
            return Err(anyhow::Error::msg(format!("Model {} not found", label)));
        }
    }
}

impl Vertex for ModelVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

// model.rs
pub trait DrawModel<'a> {
    fn draw_mesh(
        &self,
        render_pass: &mut wgpu::RenderPass,
        mesh: &'a Mesh,
        material: &'a Material,
        camera_bind_group: &'a wgpu::BindGroup,
        num_instances: u32,
    );
    fn draw_mesh_instanced(
        &self,
        render_pass: &mut wgpu::RenderPass,
        mesh: &'a Mesh,
        material: &'a Material,
        camera_bind_group: &'a wgpu::BindGroup,
        num_instances: u32,
    );
    fn draw_model(
        &self,
        render_pass: &mut wgpu::RenderPass,
        model: &str,
        camera_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_model_instanced(
        &self,
        render_pass: &mut wgpu::RenderPass,
        model: &str,
        camera_bind_group: &wgpu::BindGroup,
        instance_c_label: &str,
    );
}

impl DrawModel<'_> for ModelStore {
    fn draw_mesh(
        &self,
        render_pass: &mut wgpu::RenderPass,
        mesh: &Mesh,
        material: &Material,
        camera_bind_group: &wgpu::BindGroup,
        num_instances: u32,
    ) {
        self.draw_mesh_instanced(
            render_pass,
            mesh,
            material,
            camera_bind_group,
            num_instances,
        );
    }

    fn draw_mesh_instanced(
        &self,
        render_pass: &mut wgpu::RenderPass,
        mesh: &Mesh,
        material: &Material,
        camera_bind_group: &wgpu::BindGroup,
        num_instances: u32,
    ) {
        render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.set_bind_group(0, &material.bind_group, &[]);
        render_pass.set_bind_group(1, camera_bind_group, &[]);
        render_pass.draw_indexed(0..mesh.num_elements, 0, 0..num_instances);
    }
    fn draw_model(
        &self,
        render_pass: &mut wgpu::RenderPass,
        model: &str,
        camera_bind_group: &wgpu::BindGroup,
    ) {
        self.draw_model_instanced(render_pass, model, camera_bind_group,"one");
    }

    fn draw_model_instanced(
        &self,
        render_pass: &mut wgpu::RenderPass,
        model: &str,
        camera_bind_group: &wgpu::BindGroup,
        instance_c_label: &str,
    ) {
        let model = self.models.get(model).unwrap();
        let num_instances = self
            .set_instance_buffer(render_pass, instance_c_label)
            .unwrap();
        for mesh in &model.meshes {
            let material = &model.materials[mesh.material];
            self.draw_mesh_instanced(
                render_pass,
                mesh,
                material,
                camera_bind_group,
                num_instances,
            );
        }
    }
}
