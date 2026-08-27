use crate::{
    input_handler::InputHandler,
    plugins::Plugin,
    registry::{EventLoopRef, registry_view::PluginRegistryView, sys_runner::RenderSysRunner},
};

pub struct DefaultPlugin;

impl Plugin for DefaultPlugin {
    fn init_register(&self, register: PluginRegistryView) {}

    fn run_update_systems(&self, sys_runner: &mut crate::registry::sys_runner::SysRunner) {
        sys_runner.exec_res(sys_handle_exit);
    }

    fn run_render_systems(
        &self,
        sys_runner: &mut RenderSysRunner,
        encoder: &mut wgpu::CommandEncoder,
    ) {
    }
}
pub fn sys_handle_exit(
    (input_handler, event_loop): &mut (InputHandler, EventLoopRef),
    _: PluginRegistryView,
) {
    if input_handler.was_key_just_pressed(winit::keyboard::KeyCode::Escape) {
        event_loop.get().exit()
    }
}
