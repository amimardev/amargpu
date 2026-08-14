pub fn load_model_string(file_name: &str) -> anyhow::Result<String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets/models")
        .join(file_name);

    Ok(std::fs::read_to_string(path)?)
}
pub async fn load_binary(file_name: &str) -> anyhow::Result<Vec<u8>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("res")
        .join(file_name);
    Ok(std::fs::read(path)?)
}
