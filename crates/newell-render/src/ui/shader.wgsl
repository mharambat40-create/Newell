// One instance per grid cell; a cell is two triangles sampling one atlas slot.
struct Uniforms {
    viewport_px: vec2<f32>,
    cell_px: vec2<f32>,
    glyph_px: vec2<f32>,
    atlas_px: vec2<f32>,
    cols: u32,
    srgb_target: u32,
    hover_widget: u32,
    hover_flags: u32,
}

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var atlas: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;

const EMPTY_GLYPH: u32 = 0xFFFFFFFFu;
// Must match `WIDGET_PRESENT` / `WIDGET_LEFT_EDGE` in gpu_cell.rs.
const WIDGET_PRESENT: u32 = 1u;
const WIDGET_LEFT_EDGE: u32 = 2u;
const HOVER_COLOR: vec3<f32> = vec3<f32>(1.0, 1.0, 1.0);
// Width of the hover left-border stripe, in target pixels.
const STRIPE_PX: f32 = 1.0;

struct VsIn {
    @builtin(vertex_index) vertex: u32,
    @builtin(instance_index) instance: u32,
    @location(0) glyph: u32,
    @location(1) color: u32,
    @location(2) widget: u32,
    @location(3) flags: u32,
}

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) @interpolate(flat) origin: vec2<f32>,
    @location(2) @interpolate(flat) color: vec4<f32>,
    @location(3) @interpolate(flat) has_glyph: f32,
    @location(4) @interpolate(flat) stripe: f32,
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

    // Hover is decided here from IDs alone: the cell colours were never rewritten on the CPU.
    let owned = (in.flags & WIDGET_PRESENT) != 0u;
    let hovered = owned && (u.hover_flags & WIDGET_PRESENT) != 0u && in.widget == u.hover_widget;
    let stripe = hovered && (in.flags & WIDGET_LEFT_EDGE) != 0u;
    let has_glyph = in.glyph != EMPTY_GLYPH;

    var out: VsOut;
    // Cells with neither a glyph nor a stripe collapse outside the clip volume, so nothing is rasterised.
    out.position = select(vec4<f32>(2.0, 2.0, 2.0, 1.0), vec4<f32>(ndc, 0.0, 1.0), has_glyph || stripe);
    out.local = corner;
    out.origin = vec2<f32>(f32(in.glyph & 0xFFFFu), f32(in.glyph >> 16u));
    let base = unpack4x8unorm(in.color);
    out.color = select(base, vec4<f32>(HOVER_COLOR, base.a), hovered);
    out.has_glyph = select(0.0, 1.0, has_glyph);
    out.stripe = select(0.0, 1.0, stripe);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // Keep the sample strictly inside the slot: the atlas has no padding.
    let texel = clamp(in.local * u.glyph_px, vec2<f32>(0.01), u.glyph_px - vec2<f32>(0.01));
    var coverage = textureSample(atlas, atlas_sampler, (in.origin + texel) / u.atlas_px).r * in.has_glyph;
    // Hover left-border highlight: the leftmost pixel column of the widget's first cell.
    if in.stripe > 0.5 && in.local.x * u.cell_px.x < STRIPE_PX {
        coverage = 1.0;
    }
    var rgb = in.color.rgb;
    if u.srgb_target == 1u {
        rgb = srgb_to_linear(rgb);
    }
    return vec4<f32>(rgb, in.color.a * coverage);
}
