struct SceneUniforms {
    view_projection: mat4x4<f32>,
    light: vec4<f32>,
    viewport: vec4<f32>,
}

@group(0) @binding(0) var<uniform> scene: SceneUniforms;
@group(0) @binding(1) var surface_depth: texture_depth_2d;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) screen_distance: f32,
}

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(linear) screen_distance: f32,
}

@vertex
fn vs_main(vertex: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = scene.view_projection * vec4<f32>(vertex.position, 1.0);
    out.screen_distance = vertex.screen_distance;
    return out;
}

fn is_visible(position: vec4<f32>) -> bool {
    let pixel = vec2<i32>(position.xy);
    let size = textureDimensions(surface_depth);
    if (pixel.x < 0 || pixel.y < 0 || pixel.x >= i32(size.x) || pixel.y >= i32(size.y)) {
        return false;
    }
    let scene_depth = textureLoad(surface_depth, pixel, 0);
    let tolerance = max(0.000002, 1.5 * fwidth(position.z));
    return position.z <= scene_depth + tolerance;
}

@fragment
fn fs_visible(in: VertexOut) -> @location(0) vec4<f32> {
    if (!is_visible(in.position)) { discard; }
    return vec4<f32>(0.35, 0.75, 0.95, 1.0);
}

@fragment
fn fs_hidden(in: VertexOut) -> @location(0) vec4<f32> {
    if (is_visible(in.position) || (in.screen_distance % 10.0) >= 6.0) { discard; }
    return vec4<f32>(0.35, 0.75, 0.95, 1.0);
}
