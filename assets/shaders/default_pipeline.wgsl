// Vertex shader

struct InstanceData {
    model_matrix: mat4x4<f32>,
};

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

fn get_instance_matrix(instance_idx: u32) -> mat4x4<f32> {
    return instances[instance_idx].model_matrix;
}

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

@group(3) @binding(0)
var<uniform> loop_timer: f32;
@group(3) @binding(1)
var<uniform> global_color: vec3<f32>;

@group(2) @binding(0)
var<storage, read> instances: array<InstanceData>;

struct VertexInput {
    @builtin(instance_index) instance_idx: u32,
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    let model_matrix = get_instance_matrix(model.instance_idx);
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;

    out.clip_position = camera.view_proj * model_matrix * vec4<f32>(model.position, 1.0);
    return out;
}

// Fragment shader

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {

    var new_color = vec4(1) - vec4(global_color, 1.0) + 0.01 * loop_timer;
    var t_sampled_color = textureSample(t_diffuse, s_diffuse, in.tex_coords);

    return mix_color(t_sampled_color, new_color);
}

fn mix_color(t_sampled_color: vec4<f32>, new_color: vec4<f32>) -> vec4<f32> {
    return (0.7 * t_sampled_color + 0.3 * new_color) / 2;
}