struct SceneUniforms {
    view_projection: mat4x4<f32>,
    light: vec4<f32>,
    viewport: vec4<f32>,
    selection: vec4<f32>,
}

struct PointOut {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
}

@group(0) @binding(0) var<uniform> scene: SceneUniforms;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) corner: vec3<f32>,
) -> PointOut {
    let clip = scene.view_projection * vec4<f32>(position, 1.0);
    // Eight pixels across: four pixels from the point centre to every edge.
    let offset = corner.xy * vec2<f32>(8.0 / scene.viewport.x, 8.0 / scene.viewport.y) * clip.w;
    var out: PointOut;
    out.position = clip + vec4<f32>(offset, 0.0, 0.0);
    out.local = corner.xy;
    return out;
}

@fragment
fn fs_main(in: PointOut) -> @location(0) vec4<f32> {
    let radius = length(in.local);
    let base = vec3<f32>(0.12, 0.30, 0.46);
    if (scene.selection.y > 0.5 || scene.selection.z > 0.5) && radius > 0.57 && radius < 0.90 {
        return vec4<f32>(0.35, 0.75, 0.95, 1.0);
    }
    return vec4<f32>(min(base * scene.selection.x, vec3<f32>(1.0)), 1.0);
}
