
#[macro_export]
macro_rules! load_image {
    ($name:literal) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/images/",
            $name
        ))
    };
}

#[macro_export]
macro_rules! load_shader_str {
    ($name:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/shaders/",
            $name
        ))
    };
}