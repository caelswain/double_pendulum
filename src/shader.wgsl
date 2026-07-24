struct ViewUniforms {
    resolution: vec2<f32>, // 屏幕像素尺寸，例如 (width, height)
    _pad: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> view_uniforms: ViewUniforms;

const POSITIONS: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(-0.5, -0.5),
    vec2<f32>( 0.5, -0.5),
    vec2<f32>( 0.5,  0.5),
    vec2<f32>(-0.5, -0.5),
    vec2<f32>( 0.5,  0.5),
    vec2<f32>(-0.5,  0.5),
);


struct InstanceInput {
    @location(0) position: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) radius: f32,
    @location(3) angle: f32,
    @location(4) color: vec4<f32>,
};

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) local_pos: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) radius: f32,
    @location(3) color: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, input: InstanceInput) -> VsOut {

    var out: VsOut;

    let rotated = rotate2d(input.size * POSITIONS[vertex_index], input.angle);
    let pos = (rotated + input.position) / (view_uniforms.resolution / 2.0) - 1;
    out.position = vec4<f32>(pos, 0.0, 1.0);
    let local = input.size * POSITIONS[vertex_index];
    out.local_pos = local;
    out.size = input.size;
    out.radius = input.radius;
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let d = sdf(in.local_pos, in.size, in.radius);
    var color: vec4<f32>;
    if (d > 0.0) {
        discard;
    } else {
        color = in.color;
    }
    return color;
}

fn sdf(
    local_pos: vec2<f32>,   // px表示的相对坐标
    size: vec2<f32>,        // px表示的宽高
    radiux_px: f32,         // px表示的圆角半径
) -> f32 {
    let zero = vec2<f32>(0.0, 0.0);
    let q = abs(local_pos) - (size / 2.0 - vec2<f32>(radiux_px, radiux_px));
    let d = distance(max(q, zero), zero) + min(max(q.x, q.y), 0) - radiux_px;
    return d;
}

fn rotate2d(p: vec2<f32>, angle: f32) -> vec2<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec2<f32>(
        p.x * c - p.y * s,
        p.x * s + p.y * c,
    );
}