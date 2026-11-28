#lib_camera

struct Quad {
    origin: vec2i,
    extend: vec2u,
    edge: vec2u,
}

struct Wireframe {
    color: vec4f,
    radius: f32,
    width: f32,
    align: f32,
}

struct VertexOutput {
    @builtin(position) pos: vec4f,
    @location(0) uv: vec2f,
}

@group(1) @binding(0) var<uniform> quad: Quad;
@group(1) @binding(1) var<uniform> wireframe: Wireframe;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    let pose = vec2i(i32(index / 2), i32(index % 2));
    let extend = vec2f(quad.extend);
    let uv = vec2f(pose) + vec2f(2 * pose - 1) * vec2f(quad.edge) / extend;

    let left_down = world_to_clip(quad.origin);
    let right_up = world_to_clip(quad.origin + vec2i(quad.extend));

    var ret: VertexOutput;
    ret.pos = vec4f(left_down + (right_up - left_down) * uv, 0.0, 1.0);
    ret.uv = uv;
    return ret;
}

@fragment
fn main(in: VertexOutput) -> @location(0) vec4f {
    let halfSize = vec2f(quad.extend) * 0.5;
    let local = (in.uv - 0.5) * vec2f(quad.extend);

    let d = sdRoundRect(local, halfSize, wireframe.radius);
    let offset = (wireframe.align - 0.5) * wireframe.width;
    let dist = abs(d - offset) - wireframe.width * 0.5;

    let aa = max(fwidth(d), 0.0001);
    let alpha = 1.0 - smoothstep(-aa * 0.5, aa * 0.5, dist);
    return wireframe.color * alpha;
}

fn sdRoundRect(p: vec2f, b: vec2f, r: f32) -> f32 {
    let q = abs(p) - b + r;
    return length(max(q, vec2f())) + min(max(q.x, q.y), 0.0) - r;
}
