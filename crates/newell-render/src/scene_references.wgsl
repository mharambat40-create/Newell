struct SceneUniforms {
    view_projection: mat4x4<f32>,
    light: vec4<f32>,
    viewport: vec4<f32>,
    selection: vec4<f32>,
}

@group(0) @binding(0) var<uniform> scene: SceneUniforms;
@group(0) @binding(1) var model_depth: texture_depth_2d;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) dashed: f32,
}

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(linear) dash_distance: f32,
    @location(3) @interpolate(flat) dashed: u32,
    @location(4) @interpolate(flat) plane_visible: u32,
}

fn depth_visibility(position: vec4<f32>) -> f32 {
    let dimensions = vec2<i32>(textureDimensions(model_depth));
    let pixel = clamp(vec2<i32>(position.xy), vec2<i32>(0), dimensions - vec2<i32>(1));
    let surface_depth = textureLoad(model_depth, pixel, 0);
    // Select a single opacity for each fragment: clear scene pixels keep their
    // original alpha, while model-occluded fragments retain one fifth of it.
    return select(1.0, 0.2, position.z > surface_depth + 0.000001);
}

@vertex
fn vs_main(vertex: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = scene.view_projection * vec4<f32>(vertex.position, 1.0);
    out.normal = vertex.normal;
    out.color = vertex.color;
    let origin = scene.view_projection * vec4<f32>(0.0,0.0,0.0,1.0);
    // Screen-linear interpolation keeps dash/gap spacing constant in perspective too.
    let delta = (out.position.xy / max(out.position.w, 0.000001) - origin.xy / origin.w) * scene.viewport.xy * 0.5;
    out.dash_distance = length(delta);
    out.dashed = u32(vertex.dashed);
    out.plane_visible = 1u;
    if vertex.dashed == 0.0 && length(vertex.normal) > 0.5 {
        // An exactly edge-on plane has no visible area; suppress its collapsed
        // border too, otherwise it would fill the axis's dash gaps.
        let normal_axis = select(select(2u,1u,vertex.normal.y > 0.5),0u,vertex.normal.x > 0.5);
        let a = scene.view_projection[(normal_axis+1u)%3u] + origin;
        let b = scene.view_projection[(normal_axis+2u)%3u] + origin;
        let da = a.xy / max(a.w,0.000001) - origin.xy/origin.w;
        let db = b.xy / max(b.w,0.000001) - origin.xy/origin.w;
        out.plane_visible = select(0u,1u,abs(da.x*db.y-da.y*db.x) > 0.00001 * length(da)*length(db));
    }
    return out;
}

@fragment
fn fs_surface(in: VertexOut) -> @location(0) vec4<f32> {
    let light = 0.8 + 0.2 * max(dot(normalize(in.normal), normalize(vec3<f32>(0.4, 0.6, 0.7))), 0.0);
    let alpha = in.color.a * depth_visibility(in.position);
    return vec4<f32>(in.color.rgb * light * alpha, alpha);
}

@fragment
fn fs_line(in: VertexOut) -> @location(0) vec4<f32> {
    if in.plane_visible == 0u { discard; }
    if in.dashed != 0u && in.dash_distance % (scene.viewport.z + scene.viewport.w) >= scene.viewport.z { discard; }
    let alpha = in.color.a * depth_visibility(in.position);
    return vec4<f32>(in.color.rgb * alpha, alpha);
}
