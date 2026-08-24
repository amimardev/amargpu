mod app;
mod camera;
mod default_pipeline;
mod game_context;
mod mesh;
mod registry;
mod state;
mod texture;
use crate::{app::App, texture::Texture};

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

pub mod bg_index {
    pub const DIFFUSE: u32 = 0;
    pub const CAMERA: u32 = 1;
    pub const INSTANCES: u32 = 2;
    pub const GAME: u32 = 3;
}
pub mod keys {
    pub mod bg_layout {
        pub const INSTANCES: &str = "instances";
        pub const DIFFUSE: &str = "diffuse";
    }
    pub mod texture {
        pub const DEPTH: &str = "depth";
    }
    pub mod models {
        pub const CUBE: &str = "cube";
    }
    pub const DEFAULT: &str = "default";
    pub const ONE_INSTANCE: &str = "one";
}
