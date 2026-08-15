use std::{
    io::{BufReader, Cursor},
    path::Path,
};

use wgpu::util::DeviceExt;

use crate::{
    load_image, mesh::model::{Material, Mesh, Model, ModelVertex}, state::GlobalState, texture::Texture,
};

pub fn load_texture(
    path: &Path,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> anyhow::Result<Texture> {
    let data = std::fs::read(path)?;
    let label = path.to_string_lossy();
    Texture::from_bytes(device, queue, &data, &label)
}

pub fn load_model(
    file_name: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
) -> anyhow::Result<Model> {
    // region: load_model_string
    let model_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets/models")
        .join(file_name);
    let model_dir = model_path.parent().unwrap();
    let obj_text = std::fs::read_to_string(&model_path)?;
    // endregion

    let obj_cursor = Cursor::new(obj_text);
    let mut obj_reader = BufReader::new(obj_cursor);

    let (models, obj_materials) = tobj::load_obj_buf(
        &mut obj_reader,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
        |p| {
            let mat_text = std::fs::read_to_string(model_dir.join(p)).unwrap();
            tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
        },
    )?;

    let mut materials = Vec::new();
    for m in obj_materials? {
        let diffuse_texture = load_texture(&model_dir.join(&m.diffuse_texture), device, queue)?;
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
            ],
            label: None,
        });

        materials.push(Material {
            name: m.name,
            diffuse_texture,
            bind_group,
        })
    }

    let meshes = models
        .into_iter()
        .map(|m| {
            let vertices = (0..m.mesh.positions.len() / 3)
                .map(|i| {
                    if m.mesh.normals.is_empty() {
                        ModelVertex {
                            position: [
                                m.mesh.positions[i * 3],
                                m.mesh.positions[i * 3 + 1],
                                m.mesh.positions[i * 3 + 2],
                            ],
                            tex_coords: [
                                m.mesh.texcoords[i * 2],
                                1.0 - m.mesh.texcoords[i * 2 + 1],
                            ],
                            normal: [0.0, 0.0, 0.0],
                        }
                    } else {
                        ModelVertex {
                            position: [
                                m.mesh.positions[i * 3],
                                m.mesh.positions[i * 3 + 1],
                                m.mesh.positions[i * 3 + 2],
                            ],
                            tex_coords: [
                                m.mesh.texcoords[i * 2],
                                1.0 - m.mesh.texcoords[i * 2 + 1],
                            ],
                            normal: [
                                m.mesh.normals[i * 3],
                                m.mesh.normals[i * 3 + 1],
                                m.mesh.normals[i * 3 + 2],
                            ],
                        }
                    }
                })
                .collect::<Vec<_>>();

            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Vertex Buffer", file_name)),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Index Buffer", file_name)),
                contents: bytemuck::cast_slice(&m.mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            Mesh {
                name: file_name.to_string(),
                vertex_buffer,
                index_buffer,
                num_elements: m.mesh.indices.len() as u32,
                material: m.mesh.material_id.unwrap_or(0),
            }
        })
        .collect::<Vec<_>>();

    Ok(Model { meshes, materials })
}

/* 
pub fn load_material(glb: &GlobalState) -> anyhow::Result<Material> {
    let diffuse_bytes0 = load_image!("happy-tree.png");
    let diffuse_bytes1 = load_image!("tree.jpeg");

    let diffuse_texture0 = crate::texture::Texture::from_bytes(
        &glb.device,
        &glb.queue,
        diffuse_bytes0,
        "happy-tree.png",
    )
    .unwrap();

    let diffuse_texture1 =
        crate::texture::Texture::from_bytes(&glb.device, &glb.queue, diffuse_bytes1, "tree.jpeg")
            .unwrap();

    todo!();
    
    let diffuse_bind_group_layout = Self::get_texture_bind_group_layout(&glb.device);

    let diffuse_bind_group0 =
        Self::get_texture_bind_group(&glb.device, &diffuse_texture0, &diffuse_bind_group_layout);

    let diffuse_bind_group1 =
        Self::get_texture_bind_group(&glb.device, &diffuse_texture1, &diffuse_bind_group_layout);

    Ok(Self {
        bg_layout: diffuse_bind_group_layout,
        bgs: vec![diffuse_bind_group0, diffuse_bind_group1],
        textures: vec![diffuse_texture0, diffuse_texture1],
        active_id: 0,
    })
}
*/