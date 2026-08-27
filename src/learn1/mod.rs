mod camera;
mod default_pipeline;
mod draw_model;
mod game_context;

use amar_engine::{
    keys,
    mesh::resource::load_model,
    plugins::Plugin,
    registry::{
        registry_view::PluginRegistryView,
        sys_runner::{RenderSysRunner, SysRunner},
    },
};

use crate::{
    learn1::{
        camera::{CameraContext, sys_handle_camera},
        default_pipeline::DefaultPipeline,
        draw_model::DrawModel,
        game_context::{GameContext, sys_update_game},
    },
    other_keys,
};

pub struct Learn1Plugin;

impl Plugin for Learn1Plugin {
    fn init_register(&self, mut register: PluginRegistryView) {
        let camera_ctx = CameraContext::new(register.glb()).unwrap();
        let game_ctx = GameContext::new(register.glb()).unwrap();

        let diffuse_bg_layout = *register
            .get_by_label(keys::bg_layout::DIFFUSE)
            .first()
            .unwrap();

        let instances_bg_layout = *register
            .get_by_label(keys::bg_layout::INSTANCES)
            .first()
            .unwrap();

        let default_pipeline = DefaultPipeline::new(
            register.glb(),
            diffuse_bg_layout,
            &camera_ctx.bg_layout,
            &game_ctx.bg_layout,
            instances_bg_layout,
        )
        .unwrap();

        // initialise cube model and default,one instance_buffers
        register.spawn(
            Some(other_keys::models::CUBE),
            load_model(
                "cube/cube.obj",
                &register.glb().device,
                &register.glb().queue,
                &diffuse_bg_layout,
            )
            .unwrap(),
        );

        register.spawn(Some(amar_engine::keys::DEFAULT), default_pipeline);

        register.insert_res(camera_ctx);
        register.insert_res(game_ctx);
    }

    fn run_update_systems(&self, sys_runner: &mut SysRunner) {
        sys_runner.exec_res(sys_handle_camera);
        sys_runner.exec_res(sys_update_game);
    }

    fn run_render_systems(
        &self,
        sys_runner: &mut RenderSysRunner,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        {
            let depth_t = sys_runner.get_depth_texture();

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &sys_runner.get_surface_texture_view(),
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(sys_runner.glb().color),
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
            sys_runner.exec_render(&mut render_pass, sys_handle_render);
        }
    }
}
fn sys_handle_render(
    (game_ctx,): &(GameContext,),
    render_pass: &mut wgpu::RenderPass,
    registry: PluginRegistryView,
) {
    let default_pipeline = *registry
        .get_by_label::<wgpu::RenderPipeline>(amar_engine::keys::DEFAULT)
        .first()
        .unwrap();

    render_pass.set_pipeline(&default_pipeline);
    DefaultPipeline::bind_uniform_vars(render_pass, &game_ctx);
    registry.draw_model_instanced(
        render_pass,
        other_keys::models::CUBE,
        amar_engine::keys::DEFAULT,
    );
}
