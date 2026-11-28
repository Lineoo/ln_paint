#lib_camera

struct Quad {
    origin: vec2i,
    extend: vec2u,
}

struct VertexInput {
    @location(0) pos: vec2f,
}

struct VertexOutput {
    @builtin(position) pos: vec4f,
    // @location(0) @interpolate(flat) index: u32,
}

@group(1) @binding(0) var<uniform> quad: Quad;
@group(1) @binding(1) var<uniform> color: vec4f;

@vertex
fn vs_main(@builtin(vertex_index) index: u32, in: VertexInput) -> VertexOutput {
    var ret: VertexOutput;
    ret.pos = vec4f(world_to_clip(quad.origin) + world_to_clip_relative(vec2f(quad.extend) * in.pos), 0.0, 1.0);
    // ret.index = index;
    return ret;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    return color;
    // return vec4f(f32(in.index % 2) / 2.0, f32(in.index % 3) / 3.0, f32(in.index % 4) / 4.0, 1.0);
}
