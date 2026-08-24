use crate::bg_index;
use crate::camera::CameraContext; 

use crate::registry::ResourceRegistry;
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
        instance_c_label: &str,
    );
    fn draw_mesh_instanced(
        &self,
        render_pass: &mut wgpu::RenderPass,
        mesh: &'a Mesh,
        material: &'a Material,
        instance_c_label: &str,
    );
    fn draw_model(&self, render_pass: &mut wgpu::RenderPass, model: &str);
    fn draw_model_instanced(
        &self,
        render_pass: &mut wgpu::RenderPass,
        model: &str,
        instance_c_label: &str,
    );
}

impl DrawModel<'_> for ResourceRegistry {
    fn draw_mesh(
        &self,
        render_pass: &mut wgpu::RenderPass,
        mesh: &Mesh,
        material: &Material,
        instance_c_label: &str,
    ) {
        self.draw_mesh_instanced(render_pass, mesh, material, instance_c_label);
    }

    fn draw_mesh_instanced(
        &self,
        render_pass: &mut wgpu::RenderPass,
        mesh: &Mesh,
        material: &Material,
        instance_c_label: &str,
    ) {
        let camera_c = &self.get_res::<CameraContext>().unwrap();
        let instance_c = &self.get::<InstanceContext>(instance_c_label).unwrap();

        render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.set_bind_group(bg_index::DIFFUSE, &material.bind_group, &[]);
        render_pass.set_bind_group(bg_index::CAMERA, &camera_c.bg, &[]);
        render_pass.set_bind_group(bg_index::INSTANCES, &instance_c.bg, &[]);
        render_pass.draw_indexed(0..mesh.num_elements, 0, 0..instance_c.instances.len() as _);
    }
    fn draw_model(&self, render_pass: &mut wgpu::RenderPass, model: &str) {
        self.draw_model_instanced(render_pass, model, "one");
    }

    fn draw_model_instanced(
        &self,
        render_pass: &mut wgpu::RenderPass,
        model: &str,
        instance_c_label: &str,
    ) {
        let model = self.get::<Model>(model).unwrap();

        for mesh in &model.meshes {
            let material = &model.materials[mesh.material];
            self.draw_mesh_instanced(render_pass, mesh, material, instance_c_label);
        }
    }
}
