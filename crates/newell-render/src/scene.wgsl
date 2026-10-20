struct SceneUniforms {
    view_projection: mat4x4<f32>,
    light: vec4<f32>,
    viewport: vec4<f32>,
}

@group(0) @binding(0) var<uniform> scene: SceneUniforms;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
}

@vertex
fn vs_main(vertex: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = scene.view_projection * vec4<f32>(vertex.position, 1.0);
    out.normal = vertex.normal;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let brightness = 0.22 + 0.78 * max(dot(normalize(in.normal), normalize(scene.light.xyz)), 0.0);
    let base = vec3<f32>(0.12, 0.30, 0.46);
    let mode = u32(scene.light.w);
    if (mode == 3u) { return vec4<f32>(base, 1.0); }
    // Points and line-based modes select their geometry in Rust; their colour remains stable.
    if (mode == 0u || mode == 1u || mode == 2u || mode == 5u) { return vec4<f32>(base, 1.0); }
    return vec4<f32>(base * brightness, 1.0);
}
