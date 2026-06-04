struct ViewUniforms {
    resolution: vec2<f32>, // 屏幕像素尺寸，例如 (width, height)
    _pad: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> view_uniforms: ViewUniforms;

struct Quad {
    pos_px: vec2<f32>,     // 中心
    size_px: vec2<f32>,    // 宽高（像素）
    radius_px: f32,        // 圆角半径（像素）
    color: vec4<f32>,
};

const POSITIONS: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(-0.5, -0.5),
    vec2<f32>( 0.5, -0.5),
    vec2<f32>( 0.5,  0.5),
    vec2<f32>(-0.5, -0.5),
    vec2<f32>( 0.5,  0.5),
    vec2<f32>(-0.5,  0.5),
);

struct VertexInput {
    @location(0) local_pos: vec2<f32>,
};

struct InstanceInput {
    @location(2) position: vec2<f32>,
    @location(3) size: vec2<f32>,
    @location(4) radius: f32,
    @location(5) color: vec4<f32>,
};

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(2) color: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32,@location(2) position: vec2<f32>,@location(3) size: vec2<f32>,@location(4) radius: f32,
    @location(5) color: vec4<f32>,) -> VsOut {

    var out: VsOut;
    var x: f32;
    var y: f32;
    x = (position.x + size.x * POSITIONS[vertex_index].x) * 2 / view_uniforms.resolution.x -1.0;
    y = (position.y + size.y * POSITIONS[vertex_index].y) * 2 / view_uniforms.resolution.y -1.0;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let uv = in.position.xy / view_uniforms.resolution;
    return vec4<f32>(in.color.rgb * vec3<f32>(uv.x, uv.y, 1.0), in.color.a);
}
