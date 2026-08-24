use amarengine::{
    bg_index, keys,
    mesh::{
        instanceb::InstanceContext,
        model::{Material, Mesh, Model},
    },
    registry::registry_view::PluginRegistryView,
};

use crate::learn1::camera::CameraContext;

// model.rs
pub trait DrawModel<'a> {
    fn draw_mesh(
        &self,
        render_pass: &mut wgpu::RenderPass,
        mesh: &'a Mesh,
        material: &'a Material,
        instance_c_key: &str,
    );
    fn draw_mesh_instanced(
        &self,
        render_pass: &mut wgpu::RenderPass,
        mesh: &'a Mesh,
        material: &'a Material,
        instance_c_key: &str,
    );
    fn draw_model(&self, render_pass: &mut wgpu::RenderPass, model: &str);
    fn draw_model_instanced(
        &self,
        render_pass: &mut wgpu::RenderPass,
        model: &str,
        instance_c_key: &str,
    );
}

impl DrawModel<'_> for PluginRegistryView<'_> {
    fn draw_mesh(
        &self,
        render_pass: &mut wgpu::RenderPass,
        mesh: &Mesh,
        material: &Material,
        instance_c_key: &str,
    ) {
        self.draw_mesh_instanced(render_pass, mesh, material, instance_c_key);
    }

    fn draw_mesh_instanced(
        &self,
        render_pass: &mut wgpu::RenderPass,
        mesh: &Mesh,
        material: &Material,
        instance_c_key: &str,
    ) {
        let camera_c = self.get_res::<CameraContext>().unwrap();
        let instance_c = *self
            .get_by_label::<InstanceContext>(instance_c_key)
            .first()
            .unwrap();

        render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32); 
        material.bind(render_pass);
        camera_c.bind(render_pass);
        instance_c.bind(render_pass);
        render_pass.draw_indexed(0..mesh.num_elements, 0, 0..instance_c.instances.len() as _);
    }
    fn draw_model(&self, render_pass: &mut wgpu::RenderPass, model: &str) {
        self.draw_model_instanced(render_pass, model, keys::ONE_INSTANCE);
    }

    fn draw_model_instanced(
        &self,
        render_pass: &mut wgpu::RenderPass,
        model: &str,
        instance_c_key: &str,
    ) {
        let model = *self.get_by_label::<Model>(model).first().unwrap();

        for mesh in &model.meshes {
            let material = &model.materials[mesh.material];
            self.draw_mesh_instanced(render_pass, mesh, material, instance_c_key);
        }
    }
}
