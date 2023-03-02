struct VertIn {
    @location(0) position: vec2<f32>,
    @location(1) uv_coords: vec2<f32>, // FIXME: do we even need this?
    @location(2) alpha_scale_factor: f32,
    @location(3) color_scale_factor: f32,
    @location(4) metadata: u32,
};

struct VertOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv_coords: vec2<f32>,
    @location(1) alpha_scale_factor: f32,
    @location(2) color_scale_factor: f32,
    @location(3) metadata: u32,
};

@vertex
fn main_vert(in: VertIn) -> VertOut {
    var out: VertOut;
    out.clip_position = vec4<f32>(in.position, 0.0, 1.0); // FIXME: should these two values actually be 0.0 and 1.0?
    out.uv_coords = in.uv_coords;
    out.alpha_scale_factor = in.alpha_scale_factor;
    out.color_scale_factor = in.color_scale_factor;

    return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn main_frag(in: VertOut) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.uv_coords) * vec4<f32>(in.color_scale_factor, in.color_scale_factor, in.color_scale_factor, in.alpha_scale_factor);
}
