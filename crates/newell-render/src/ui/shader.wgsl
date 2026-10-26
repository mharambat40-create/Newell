// One instance per grid cell; a cell is two triangles sampling one atlas slot.
// Semantic ink is fixed per cell; interaction state never changes the palette.
struct Uniforms {
    viewport_px: vec2<f32>,
    cell_px: vec2<f32>,
    glyph_px: vec2<f32>,
    atlas_px: vec2<f32>,
    cols: u32,
    srgb_target: u32,
    inks: array<vec4<f32>, 3>,
}

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var atlas: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;

const EMPTY_GLYPH: u32 = 0xFFFFFFFFu;

struct VsIn {
    @builtin(vertex_index) vertex: u32,
    @builtin(instance_index) instance: u32,
    @location(0) glyph: u32,
    @location(1) ink: u32,
}

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) @interpolate(flat) origin: vec2<f32>,
    @location(2) @interpolate(flat) has_glyph: f32,
    @location(3) @interpolate(flat) ink: u32,
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(high, low, c <= vec3<f32>(0.04045));
}

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
    );
    let corner = corners[in.vertex];
    let cols = max(u.cols, 1u);
    let cell = vec2<f32>(f32(in.instance % cols), f32(in.instance / cols));
    let px = (cell + corner) * u.cell_px;
    let ndc = vec2<f32>(px.x / u.viewport_px.x * 2.0 - 1.0, 1.0 - px.y / u.viewport_px.y * 2.0);

    let has_glyph = in.glyph != EMPTY_GLYPH;

    var out: VsOut;
    // A cell without a glyph is transparent and collapses outside the clip volume, so nothing
    // is rasterised.
    out.position = select(vec4<f32>(2.0, 2.0, 2.0, 1.0), vec4<f32>(ndc, 0.0, 1.0), has_glyph);
    out.local = corner;
    out.origin = vec2<f32>(f32(in.glyph & 0xFFFFu), f32(in.glyph >> 16u));
    out.has_glyph = select(0.0, 1.0, has_glyph);
    out.ink = select(0u, in.ink, in.ink < 3u);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // Keep the sample strictly inside the slot: the atlas has no padding.
    let texel = clamp(in.local * u.glyph_px, vec2<f32>(0.01), u.glyph_px - vec2<f32>(0.01));
    let coverage = textureSample(atlas, atlas_sampler, (in.origin + texel) / u.atlas_px).r * in.has_glyph;
    // Foreground glyph over whatever is behind it; the alpha is the glyph's coverage, which is
    // the only thing that makes an edge pixel differ from pure foreground.
    var rgb = u.inks[in.ink].rgb;
    let alpha = coverage;
    if u.srgb_target == 1u {
        rgb = srgb_to_linear(rgb);
    }
    return vec4<f32>(rgb, alpha);
}
