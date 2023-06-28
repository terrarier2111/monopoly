// Documentation see: https://gpuweb.github.io/gpuweb/wgsl/

struct CameraUniform {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normals: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

struct InstanceInput {
    @location(3) model_matrix_0: vec4<f32>,
    @location(4) model_matrix_1: vec4<f32>,
    @location(5) model_matrix_2: vec4<f32>,
    @location(6) model_matrix_3: vec4<f32>,
};

@vertex
fn main_vert(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
            instance.model_matrix_0,
            instance.model_matrix_1,
            instance.model_matrix_2,
            instance.model_matrix_3,
       );
    var out: VertexOutput;
    let world_position = model_matrix * vec4<f32>(model.position, 1.0);
    out.clip_position = camera.view_proj * (world_position + camera.view_pos);
    out.tex_coords = model.tex_coords;
    return out;
}


@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

@fragment
fn main_frag(
    in: VertexOutput
) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}
