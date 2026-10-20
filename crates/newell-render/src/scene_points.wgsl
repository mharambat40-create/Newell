struct SceneUniforms {
    view_projection: mat4x4<f32>,
    light: vec4<f32>,
    viewport: vec4<f32>,
}

@group(0) @binding(0) var<uniform> scene: SceneUniforms;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) corner: vec3<f32>,
) -> @builtin(position) vec4<f32> {
    let clip = scene.view_projection * vec4<f32>(position, 1.0);
    // Eight pixels across: four pixels from the point centre to every edge.
    let offset = corner.xy * vec2<f32>(8.0 / scene.viewport.x, 8.0 / scene.viewport.y) * clip.w;
    return clip + vec4<f32>(offset, 0.0, 0.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> { return vec4<f32>(0.12, 0.30, 0.46, 1.0); }
