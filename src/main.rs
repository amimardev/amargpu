mod helper_macros;
mod learn1;

pub mod other_bg_index {
    pub const GAME: u32 = 3;
}
pub mod other_keys {
    pub mod models {
        pub const CUBE: &str = "cube";
    }
}

use crate::learn1::Learn1Plugin;
use amar_engine::app::App;
use amar_engine::plugins::default::DefaultPlugin;

fn main() {
    let _ = App::new()
        .register(Learn1Plugin)
        .register(DefaultPlugin)
        .run();
}
