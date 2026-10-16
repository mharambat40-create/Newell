struct SceneUniforms {
    view_projection: mat4x4<f32>,
    light: vec4<f32>,
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
    return vec4<f32>(vec3<f32>(0.12, 0.30, 0.46) * brightness, 1.0);
}
