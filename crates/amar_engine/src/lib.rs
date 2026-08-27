pub mod app; 
pub mod input_handler;
pub mod mesh;
pub mod plugins;
pub mod registry;
pub mod state;
pub mod texture;
mod ecs;

pub mod bg_index {
    pub const DIFFUSE: u32 = 0;
    pub const CAMERA: u32 = 1;
    pub const INSTANCES: u32 = 2; 
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
    }
    pub const DEFAULT: &str = "default";
    pub const ONE_INSTANCE: &str = "one";
}
