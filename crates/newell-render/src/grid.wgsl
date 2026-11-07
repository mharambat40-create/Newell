// SPDX-License-Identifier: AGPL-3.0-or-later
struct Inputs { inverse: mat4x4<f32>, vp: mat4x4<f32>, viewport: vec4<f32>, ink: vec4<f32>, eye: vec4<f32> }
@group(0) @binding(0) var<uniform> u: Inputs;
@group(0) @binding(1) var depth: texture_depth_2d;
@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
}
fn coverage(p: vec2<f32>, footprint: vec2<f32>, spacing: f32) -> f32 {
    let d = abs(fract(p / spacing + 0.5) - 0.5) * spacing;
    let pixel_distance = d / max(footprint, vec2<f32>(1e-20));
    let line = 1.0 - smoothstep(vec2<f32>(0.0), vec2<f32>(1.0), pixel_distance);
    let density = smoothstep(vec2<f32>(2.0), vec2<f32>(12.0), spacing / max(footprint, vec2<f32>(1e-20)));
    return max(line.x * density.x, line.y * density.y);
}
@fragment fn fs(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let ndc = (frag.xy - u.viewport.xy) / u.viewport.zw * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0);
    let a = u.inverse * vec4<f32>(ndc, 0.0, 1.0);
    let b = u.inverse * vec4<f32>(ndc, 0.5, 1.0);
    let origin = select(a.xyz / a.w, u.eye.xyz, u.eye.w > 0.5);
    let ray = b.xyz / b.w - origin;
    let rz = select(ray.z, 1e-20, abs(ray.z) < 1e-20);
    let t = -origin.z / rz;
    let world = origin + t * ray;
    // Differentiate the ray/plane quotient analytically. Finite differences of
    // world hits make spacing depend on raster derivative quads near the horizon.
    let ox = dpdx(origin); let oy = dpdy(origin);
    let rx = dpdx(ray); let ry = dpdy(ray);
    let wx = ox - (ox.z * ray + origin.z * rx) / rz + origin.z * ray * rx.z / (rz * rz);
    let wy = oy - (oy.z * ray + origin.z * ry) / rz + origin.z * ray * ry.z / (rz * rz);
    let footprint = sqrt(wx.xy * wx.xy + wy.xy * wy.xy);
    let desired = max(u.ink.y * 90.0, 1e-12);
    let decade = pow(10.0, floor(log2(desired) / log2(10.0)));
    let ratio = desired / decade;
    var low = decade;
    var high = 2.0 * decade;
    if ratio >= 5.0 { low = 5.0 * decade; high = 10.0 * decade; }
    else if ratio >= 2.0 { low = 2.0 * decade; high = 5.0 * decade; }
    let blend = smoothstep(0.0, 1.0, log2(desired / low) / log2(high / low));
    let alpha = mix(coverage(world.xy, footprint, low), coverage(world.xy, footprint, high), blend);
    let clip = u.vp * vec4<f32>(world, 1.0);
    let z = clip.z / clip.w;
    let horizon = smoothstep(0.005, 0.08, abs(ray.z) / length(ray));
    let surface_depth = textureLoad(depth, vec2<i32>(frag.xy), 0);
    // Clear depth means no model. The infinite grid is independent of model clip planes.
    if abs(ray.z) < 1e-20 || t <= 0.0 || clip.w <= 0.0 || (surface_depth < 1.0 && z >= surface_depth) { discard; }
    return vec4<f32>(vec3<f32>(u.ink.x), alpha * 0.20 * horizon);
}
