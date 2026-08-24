mod learn1;
mod helper_macros;

pub mod other_bg_index {
    pub const GAME: u32 = 3;
}
pub mod other_keys {
    pub mod models {
        pub const CUBE: &str = "cube";
    }
}


use amarengine::app::App;
use amarengine::plugins::default::DefaultPlugin;
use crate::learn1::Learn1Plugin;

fn main() {
    let _ = App::new()
        .register(Learn1Plugin)
        .register(DefaultPlugin)
        .run();
}
