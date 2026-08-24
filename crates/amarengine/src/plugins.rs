pub mod default;
use wgpu::SurfaceTexture;

use crate::{
    registry::{ 
        registry_view::PluginRegistryView,
        sys_runner::{RenderSysRunner, SysRunner},
    },
    state::State,
};

pub trait Plugin {
    fn init_register(&self, register: PluginRegistryView);
    fn run_update_systems(&self, sys_runner: &mut SysRunner);
    fn run_render_systems(&self, sys_runner: &mut RenderSysRunner, encoder: &mut wgpu::CommandEncoder);
}

pub trait PluginHandler { 
    fn init_all(&mut self);
    fn run_update_all(&mut self);
    fn run_render_all(
        &mut self,
        surface_texture: &SurfaceTexture,
        command_encoder: &mut wgpu::CommandEncoder,
    );
}

impl PluginHandler for State {
 

    /// Call once at startup, after all plugins are registered.
    fn init_all(&mut self) {
        for plugin in &self.plugins {
            plugin.init_register(self.registry.view());
        }
    }

    fn run_update_all(&mut self) {
        let sys_runner = &mut SysRunner::new(&mut self.registry);
        for plugin in &self.plugins {
            plugin.run_update_systems(sys_runner);
        }
    }

    fn run_render_all(
        &mut self,
        surface_texture: &SurfaceTexture,
        command_encoder: &mut wgpu::CommandEncoder,
    ) {
        let sys_runner = &mut RenderSysRunner::new(&mut self.registry, surface_texture);
        for plugin in &self.plugins {
            plugin.run_render_systems(sys_runner, command_encoder);
        }
    }
}
