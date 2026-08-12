mod app;
mod state;
mod buffer;
mod texture;
mod camera;
mod basic_pipeline;
mod uniform_vars;
use crate::app::App;

use winit::event_loop::EventLoop;

pub fn run() -> anyhow::Result<()> {
    env_logger::init();

    let event_loop = EventLoop::with_user_event().build()?;

    let mut app = App::new();
    event_loop.run_app(&mut app)?;

    Ok(())
}

#[macro_export]
macro_rules! load_image {
    ($name:literal) => {
        include_bytes!(concat!("../assets/images/", $name))
    };
}

#[macro_export]
macro_rules! load_shader_str {
    ($name:literal) => {
        include_str!(concat!("../assets/shaders/", $name))
    };
}