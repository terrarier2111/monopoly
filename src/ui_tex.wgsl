struct VertIn {
    @location(0) position: vec2<f32>,
    @location(1) uv_coords: vec2<f32>,
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
    out.metadata = in.metadata;

    return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

const GRAYSCALE_CONV = 1; // 1 << 0

@fragment
fn main_frag(in: VertOut) -> @location(0) vec4<f32> {
    let sampled1 = textureSample(t_diffuse, s_diffuse, in.uv_coords) * vec4<f32>(in.color_scale_factor, in.color_scale_factor, in.color_scale_factor, in.alpha_scale_factor);

    /*let bits = extractBits(in.metadata, u32(GRAYSCALE_CONV), u32(GRAYSCALE_CONV));
    if bits != u32(0) {
        let all = sampled[0] + sampled[1] + sampled[2];
        let avg = all / 3.0;
        return vec4<f32>(avg, avg, avg, sampled[3]);
    } else {
        return sampled;
    }*/
    let sampled = vec4<f32>(sampled1[0], sampled1[1], sampled1[2], sampled1[3]);
    if in.metadata != u32(0) {
        let all = sampled[0] + sampled[1] + sampled[2];
        let avg = all / 3.0;
        return vec4<f32>(avg, avg, avg, sampled[3]);
    } else {
        return sampled;
    }
}
