// SPDX-License-Identifier: AGPL-3.0-or-later
struct Inputs { inverse: mat4x4<f32>, vp: mat4x4<f32>, viewport: vec4<f32>, ink: vec4<f32>, eye: vec4<f32> }
@group(0) @binding(0) var<uniform> u: Inputs;
@group(0) @binding(1) var depth: texture_depth_2d;
@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
}
fn line_coverage(distance: vec2<f32>, width: vec2<f32>, softness: f32) -> vec2<f32> {
    let radius = 0.5 * width;
    return 1.0 - smoothstep(radius - softness, radius + softness, distance);
}

fn coverage(p: vec2<f32>, footprint: vec2<f32>, spacing: f32, progress: f32, softness: f32) -> f32 {
    let coordinate = p / spacing;
    let shared_index = floor(coordinate / 5.0 + 0.5);
    let shared_position = 5.0 * shared_index;
    let nearest = floor(coordinate + 0.5);
    // When the nearest fine index is shared, use its closer inserted neighbor.
    let neighbor = shared_position + select(vec2<f32>(-1.0), vec2<f32>(1.0), coordinate >= shared_position);
    let inserted = select(nearest, neighbor, nearest == shared_position);

    let insertion = smoothstep(0.15, 0.85, progress);
    let major = select(shared_index == 5.0 * floor(shared_index / 5.0), vec2<bool>(true), progress >= 0.5);
    let shared_width = select(vec2<f32>(1.0), vec2<f32>(1.25), major);
    let pitch_pixels = spacing / max(footprint, vec2<f32>(1e-20));
    let fine_density = smoothstep(vec2<f32>(2.0), vec2<f32>(12.0), pitch_pixels);
    let coarse_density = smoothstep(vec2<f32>(2.0), vec2<f32>(12.0), 5.0 * pitch_pixels);
    let shared_coverage = line_coverage(abs(coordinate - shared_position) * pitch_pixels, shared_width, softness);
    let inserted_coverage = line_coverage(abs(coordinate - inserted) * pitch_pixels, vec2<f32>(1.0), softness);
    let axis = max(
        shared_coverage * mix(coarse_density, fine_density, insertion),
        inserted_coverage * fine_density * insertion,
    );
    return max(axis.x, axis.y);
}
@fragment fn fs(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let ndc = (frag.xy - u.viewport.xy) / u.viewport.zw * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0);
    let a = u.inverse * vec4<f32>(ndc, 0.0, 1.0);
    let b = u.inverse * vec4<f32>(ndc, 0.5, 1.0);
    let origin = select(a.xyz / a.w, u.eye.xyz, u.eye.w > 0.5);
    let ray = b.xyz / b.w - origin;
    let parallel = abs(ray.z) < 1e-20;
    let rz = select(ray.z, 1.0, parallel);
    let t = -origin.z / rz;
    let world = origin + t * ray;
    // Differentiate the ray/plane quotient analytically. Finite differences of
    // world hits make spacing depend on raster derivative quads near the horizon.
    let ox = dpdx(origin); let oy = dpdy(origin);
    let rx = dpdx(ray); let ry = dpdy(ray);
    let wx = ox - (ox.z * ray + origin.z * rx) / rz + origin.z * ray * rx.z / (rz * rz);
    let wy = oy - (oy.z * ray + origin.z * ry) / rz + origin.z * ray * ry.z / (rz * rz);
    let footprint = sqrt(wx.xy * wx.xy + wy.xy * wy.xy);
    let desired = max(u.ink.y * 120.0, 1e-12);
    let level = log2(desired) / log2(5.0);
    let fine = pow(5.0, floor(level));
    let progress = 1.0 - fract(level);
    let ray_length = length(ray);
    let distance = t * ray_length;
    let orthographic = u.eye.w <= 0.5;
    let origin_depth = dot(-u.eye.xyz, ray) / ray_length;
    let center_depth = max(-u.eye.z * ray_length / rz, 0.0);
    let grid_origin = u.vp[3];
    let origin_ndc_y = grid_origin.y / max(grid_origin.w, 1e-20);
    let origin_y = u.viewport.y + (0.5 - 0.5 * origin_ndc_y) * u.viewport.w;

    var fade_distance = distance - select(0.0, center_depth, orthographic);
    var edge_distance = distance;
    var angle = abs(ray.z) / ray_length;
    // Use the stronger fade at each pair of mirrored screen positions.
    // The auxiliary projection makes the ray inputs affine in screen Y.
    if abs(origin_ndc_y) <= 1.0 && grid_origin.w > 0.0
        && (!orthographic || origin_depth > 1e-6) {
        let offset = 2.0 * (origin_y - frag.y);
        let reflected_ray = ray + offset * ry;
        let reflected_length = length(reflected_ray);
        let reflected_parallel = abs(reflected_ray.z) < 1e-20;
        let reflected_z = select(reflected_ray.z, 1.0, reflected_parallel);
        let reflected_distance = -(origin.z + offset * oy.z) * reflected_length / reflected_z;
        let reflected_angle = abs(reflected_ray.z) / max(reflected_length, 1e-20);
        let reflected_visible = !reflected_parallel && reflected_distance > 0.0;
        angle = min(angle, select(0.0, reflected_angle, reflected_visible));
        edge_distance = min(distance, reflected_distance);
        fade_distance = max(distance, reflected_distance) - select(0.0, origin_depth, orthographic);
    }

    let view_height = max(u.ink.y * u.viewport.w, 1e-12);
    let distance_fade = smoothstep(3.0, 8.0, fade_distance / view_height);
    let softness = mix(0.30, 0.50, distance_fade);
    let alpha = coverage(world.xy, footprint, fine, progress, softness);
    let horizon = smoothstep(0.005, 0.08, angle);
    let gradient = vec2<f32>(dpdx(distance), dpdy(distance));
    // In a nearly face-on view, keep a finite fade in world units.
    let width = max(48.0 * length(gradient), max(u.ink.y, 1e-12));
    let forward_fade = select(1.0, smoothstep(0.0, width, edge_distance), orthographic);
    let clip = u.vp * vec4<f32>(world, 1.0);
    let surface_depth = textureLoad(depth, vec2<i32>(frag.xy), 0);
    // Clear depth means no model. The infinite grid is independent of model clip planes.
    if parallel || t <= 0.0 || clip.w <= 0.0 || (surface_depth < 1.0 && clip.z / clip.w >= surface_depth) { discard; }
    return vec4<f32>(vec3<f32>(u.ink.x), alpha * 0.20 * horizon * forward_fade * (1.0 - distance_fade));
}
