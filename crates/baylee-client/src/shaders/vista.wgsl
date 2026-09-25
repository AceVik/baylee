// The front door's scene: the cleft (#295).
//
// A blue-hour landscape in depth, drawn as arithmetic for the reason
// `docs/legal.md` §2 gives about every surface in this client: ornament is
// the easiest thing to borrow by accident, and value noise, distance fields
// and hashed points borrow nothing. It grows out of `ambience.wgsl`: the same
// warped field and aurora make its sky, and the same motes and grain its air.
//
// Five layers, back to front, each moving with the pointer by its own share
// (the nearer, the more), which is the whole of the depth:
//
// - the sky, from the lobby's own dark at the top to a horizon with a dying
//   ember in it, clouded by the warped field, with the aurora's bands and,
//   later in the hour, stars;
// - a far ridge of crystal fins, one skyline from two noise samples;
// - the table's river of light, narrow where it leaves the ridge and wide
//   where it runs under the panel;
// - the cleft: two broken mineral jambs around the panel, leaning inwards and
//   never meeting, lit along their inner edge (champagne bevel, violet glow,
//   sparks). On a tall screen the stone is a sill and a broken lintel instead;
// - the near air: motes, grain, and a dither against banding.
//
// The scene is framed by the panel it stands behind (`params.panel`), and
// the ring round the panel is held dark whatever else is lit, so the leather's
// edge always reads.
//
// Passing through the cleft (choosing a gateway) is two gates: `gate_a` is
// the one the viewer stands before and walks through, `gate_b` the one they
// arrive inside. `vista.rs` schedules the uniforms; this file only draws a
// frame of them.

#import bevy_render::globals::Globals
#import bevy_ui::ui_vertex_output::UiVertexOutput
#import "embedded://baylee_client/shaders/noise.wgsl"::{noise2, hash2}

struct VistaParams {
    /// The panel the scene frames: centre xy and half extent zw, in the
    /// field's units (the height is 1, the width the aspect; y runs down).
    panel: vec4<f32>,
    /// Pointer xy (-1 to 1, low-passed on the CPU), aspect, energy (0 stills
    /// the scene: no time and no pointer).
    view: vec4<f32>,
    /// The hour (0 the gateway's side, 1 the far side), haze, glow, quality
    /// (0 drops what a phone's tiler pays most for).
    hour: vec4<f32>,
    /// The gate walked through: opening, alpha, zoom, dolly (0 to 1).
    gate_a: vec4<f32>,
    /// The gate arrived in: opening, alpha, zoom, unused.
    gate_b: vec4<f32>,
    /// River brightness, spark rate, the scene's own alpha, the river's zoom.
    air: vec4<f32>,
}

@group(0) @binding(1) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> params: VistaParams;

// The palette, linear. The sRGB each came from is beside it: the lobby's own
// dark and accent, and the table's champagne, verdigris and violet.
const TOP: vec3<f32> = vec3<f32>(0.0014, 0.0027, 0.0085); // (0.018, 0.035, 0.09)
const HORIZON: vec3<f32> = vec3<f32>(0.0174, 0.0272, 0.1329); // (0.14, 0.18, 0.40)
const HORIZON_LATE: vec3<f32> = vec3<f32>(0.0049, 0.0072, 0.0331); // (0.06, 0.08, 0.20)
const EMBER: vec3<f32> = vec3<f32>(0.1065, 0.0331, 0.0220); // (0.36, 0.20, 0.16)
const AURORA: vec3<f32> = vec3<f32>(0.0637, 0.2049, 0.7874); // (0.28, 0.49, 0.90)
const RIDGE: vec3<f32> = vec3<f32>(0.0039, 0.0060, 0.0220); // (0.05, 0.07, 0.16)
const TIPS: vec3<f32> = vec3<f32>(0.0134, 0.0220, 0.0732); // (0.12, 0.16, 0.30)
const CORE: vec3<f32> = vec3<f32>(0.2633, 0.4480, 1.0000); // (0.55, 0.70, 1.00)
const CORE_LATE: vec3<f32> = vec3<f32>(0.0732, 0.3424, 0.3185); // (0.30, 0.62, 0.60)
const VIOLET: vec3<f32> = vec3<f32>(0.1473, 0.0732, 0.4770); // (0.42, 0.30, 0.72)
const SHALLOWS: vec3<f32> = vec3<f32>(0.0397, 0.2330, 0.1960); // (0.22, 0.52, 0.48)
const STONE: vec3<f32> = vec3<f32>(0.0049, 0.0044, 0.0066); // (0.06, 0.055, 0.075)
const CHAMPAGNE: vec3<f32> = vec3<f32>(0.5705, 0.3931, 0.1626); // (0.78, 0.66, 0.44)
const GLOW: vec3<f32> = vec3<f32>(0.2633, 0.1005, 0.8900); // (0.55, 0.35, 0.95)
const SPARK: vec3<f32> = vec3<f32>(0.8900, 0.6921, 0.3185); // (0.95, 0.85, 0.60)

/// Where the sky meets the ridge, from the top.
const Y_H: f32 = 0.30;
/// How far the cleft stands off the panel, per unit of opening.
const MARGIN: vec2<f32> = vec2<f32>(0.09, 0.14);
/// tan 4°: how far the jambs lean in.
const LEAN: f32 = 0.07;
/// The brightest the ring round the panel may be, linear luminance.
const RING_CAP: f32 = 0.12;

fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

/// Signed distance to a box of half extent `b` with corners of radius `r`.
fn rounded_box(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - b + vec2<f32>(r, r);
    return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - r;
}

/// The skyline's three samples at `x`: the broad noise, the fine noise and
/// the fin. The jambs' broken tops are cut from the same line.
struct Skyline {
    broad: f32,
    fine: f32,
    fin: f32,
}

fn skyline(x: f32, quality: f32) -> Skyline {
    var s: Skyline;
    s.broad = noise2(vec2<f32>(x * 3.1, 7.0));
    s.fine = 0.5;
    if quality > 0.5 {
        s.fine = noise2(vec2<f32>(x * 11.0, 9.0));
    }
    // A triangle wave: 0 at a fin's point, 1 between two.
    s.fin = abs(fract(x * 9.0 + s.fine) - 0.5) * 2.0;
    return s;
}

fn skyline_height(s: Skyline) -> f32 {
    return Y_H + 0.09 * s.broad + 0.03 * s.fin;
}

/// What one gate adds at a point.
struct Gate {
    /// How much of the point is stone.
    stone: f32,
    /// The light on it: bevel, glow and sparks, already coloured.
    light: vec3<f32>,
}

/// The cleft at `at` (the stone's own parallax applied), for a gate of
/// `opening` seen at `zoom`.
fn gate(at: vec2<f32>, opening: f32, zoom: f32, top: f32, field: f32, t: f32, px: f32) -> Gate {
    let centre = params.panel.xy;
    let aspect = params.view.z;
    // Scaled about the panel's centre: walking through is the gate growing
    // past the viewer while the panel stays where it is.
    var g = (at - centre) / zoom;
    // Leaning in: the higher, the nearer the middle.
    g.x = g.x - sign(g.x) * LEAN * g.y;
    let half = params.panel.zw + MARGIN * opening;
    // Distances back in screen units, so edges keep their width at any zoom.
    let d = rounded_box(g, half, 0.12) * zoom;

    // Which part of the rim is stone: the sides on a wide screen, a sill and
    // a lintel on a tall one.
    let angle = atan2(g.y, g.x);
    let landscape = smoothstep(0.55, 0.75, abs(cos(angle)));
    let portrait = smoothstep(0.55, 0.75, abs(sin(angle)));
    let side = mix(portrait, landscape, smoothstep(0.8, 1.25, aspect));
    // Broken off at the top along the skyline's own line, raised.
    let whole = smoothstep(top - px, top + px, at.y);
    let mask = side * whole;

    var out: Gate;
    out.stone = smoothstep(-px, px, d) * mask;
    // A thin worked edge on the stone's side of the rim, crisp towards the
    // opening and softer into the stone.
    let bevel = smoothstep(-px, px, d) * (1.0 - smoothstep(0.0015, 0.006, d));
    // Tight: a long tail of violet reads as a lit curtain, not as stone
    // catching the light at its edge.
    let glow = select(0.0, exp(-d / 0.025), d > 0.0) * (0.9 + 0.1 * field);
    out.light = (CHAMPAGNE * 0.5 * bevel + GLOW * 0.18 * glow * params.hour.z) * mask;

    // Sparks on the band just outside the rim, rising and burning out.
    let band = smoothstep(0.0, 0.004, d) * (1.0 - smoothstep(0.02, 0.03, d)) * mask;
    if band > 0.0 {
        let rising = (at + vec2<f32>(0.0, t * 0.06)) * 60.0;
        let cell = floor(rising);
        let seed = hash2(cell);
        let lit = step(1.0 - 0.06 * params.air.y, seed);
        let point = fract(rising) - vec2<f32>(seed, fract(seed * 7.3));
        let life = fract(t * 0.7 + seed * 13.1);
        let spark = (1.0 - smoothstep(0.03, 0.14, length(point))) * sin(3.14159 * life);
        out.light = out.light + SPARK * spark * lit * band;
    }
    return out;
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let energy = params.view.w;
    let t = globals.time * energy;
    let aspect = params.view.z;
    let quality = params.hour.w;
    let hour = params.hour.x;
    let centre = params.panel.xy;
    let px = 1.0 / max(in.size.y, 1.0);
    let uv = in.uv * vec2<f32>(aspect, 1.0);
    let pointer = params.view.xy * energy;
    // Two slow drifts that never line up, so nothing repeats within a minute.
    let drift = vec2<f32>(sin(t * 0.126), cos(t * 0.188) * 0.7);

    // The sky, which barely moves.
    let sky_at = uv + pointer * 0.004 + drift * 0.003;
    let warp = vec2<f32>(
        noise2(sky_at * 1.6 + vec2<f32>(t * 0.05, t * -0.03)),
        noise2(sky_at * 1.6 + vec2<f32>(-t * 0.04, t * 0.06) + 4.0),
    );
    let field = noise2(sky_at * 2.2 + warp * 1.4 + vec2<f32>(0.0, t * 0.02));
    let horizon = mix(HORIZON, HORIZON_LATE, hour);
    var rgb = mix(TOP, horizon, smoothstep(0.0, Y_H, sky_at.y));
    rgb = rgb * (0.8 + 0.4 * field) + AURORA * field * 0.02;
    // Just above the ridge's lowest saddles, where it can be seen.
    let ember = exp(-pow((sky_at.y - Y_H + 0.02) / 0.06, 2.0)) * (1.0 - hour);
    rgb = rgb + EMBER * 0.30 * ember;
    let above = 1.0 - smoothstep(Y_H - 0.12, Y_H, sky_at.y);
    let ribbon = sin((sky_at.y * 6.0 + warp.x * 3.0 - t * 0.35) * 3.14159);
    let bands = pow(max(ribbon, 0.0), 6.0) * (0.35 + 0.65 * field);
    rgb = rgb + AURORA * bands * 0.05 * mix(0.5, 1.0, hour) * above;
    if hour > 0.0 {
        let cells = sky_at * 90.0;
        let seed = hash2(floor(cells));
        let point = fract(cells) - vec2<f32>(seed, fract(seed * 7.3));
        let star = (1.0 - smoothstep(0.02, 0.07, length(point))) * step(0.992, seed);
        let twinkle = 0.6 + 0.4 * sin(t * 1.3 + seed * 40.0);
        rgb = rgb + vec3<f32>(0.6, 0.7, 1.0) * star * twinkle * hour * above * 0.5;
    }

    // The ridge. On the far side it is another stretch of the same line.
    let dolly = params.gate_a.w;
    var ridge_at = uv + pointer * 0.012 + drift * 0.006;
    ridge_at = centre + (ridge_at - centre) / (1.0 + 0.06 * dolly);
    let ridge_x = select(ridge_at.x, 3.7 - ridge_at.x, hour > 0.5);
    let line = skyline(ridge_x, quality);
    let skyline_y = skyline_height(line);
    let ground = smoothstep(skyline_y - px, skyline_y + px, ridge_at.y);
    let rim = 1.0 - smoothstep(0.0, 6.0 * px, ridge_at.y - skyline_y);
    let body = mix(TIPS, RIDGE, smoothstep(skyline_y, skyline_y + 0.12, ridge_at.y));
    rgb = mix(rgb, body + horizon * 0.5 * rim, ground);

    // The river, running out of the ridge towards the viewer.
    var river_at = uv + pointer * 0.020 + drift * 0.010;
    river_at = centre + (river_at - centre) / params.air.w;
    let depth = clamp((river_at.y - Y_H) / (1.0 - Y_H), 0.0, 1.0);
    let width = mix(0.02, 0.30, depth * depth);
    let offset = (river_at.x - centre.x - 0.052 * (warp.x - 0.5)) / width;
    let course = exp(-offset * offset) * ground;
    // Only near the river: beyond four widths neither it nor its banks add
    // anything visible.
    if ground > 0.0 && abs(offset) < 4.0 {
        let flow = noise2(vec2<f32>((river_at.y - t * 0.18) * 5.0, river_at.x * 9.0));
        var sparkle = 0.0;
        if quality > 0.5 {
            sparkle = pow(noise2(river_at * 14.0 + warp * 2.0 - vec2<f32>(0.0, t * 0.3)), 4.0);
        }
        let core = mix(CORE, CORE_LATE, 0.4 * hour);
        let banks = max(exp(-offset * offset / 3.24) - course, 0.0) * ground;
        let shallows = SHALLOWS * (1.0 - depth) * course * 0.15;
        // Fading before the foot of the screen, where the small print is.
        let foot = 1.0 - 0.6 * smoothstep(0.78, 1.0, uv.y);
        rgb = rgb + core * 0.35 * params.air.x * course * (0.6 + 0.4 * flow + 0.6 * sparkle) * foot;
        rgb = rgb + VIOLET * 0.12 * banks + shallows;
    }

    // The cleft: the gate walked through, and the one arrived in.
    let stone_at = uv + pointer * 0.035;
    let top = skyline_y - 0.22;
    let veins = mix(AURORA, VIOLET, hour) * field * mix(0.06, 0.08, hour);
    // Strata: the field again, so the stone is not a flat cut-out.
    let stone = STONE * (0.7 + 0.6 * field) + veins;
    // Veins: a ridged line through the stone, one more sample and only where
    // there is stone to see it on.
    let near_stone = rounded_box(stone_at - centre, params.panel.zw, 0.02) > 0.05;
    var vein = 0.0;
    if near_stone {
        let grain_at = stone_at * vec2<f32>(3.0, 7.0) + warp * 0.8;
        vein = 1.0 - smoothstep(0.0, 0.035, abs(noise2(grain_at) - 0.5));
    }
    let stone_lit = stone + mix(CHAMPAGNE, VIOLET, 0.5 + 0.5 * hour) * vein * 0.018;
    if params.gate_a.y > 0.0 {
        let a = gate(stone_at, params.gate_a.x, params.gate_a.z, top, field, t, px);
        rgb = mix(rgb, stone_lit, a.stone * params.gate_a.y) + a.light * params.gate_a.y;
    }
    if params.gate_b.y > 0.0 {
        let b = gate(stone_at, params.gate_b.x, params.gate_b.z, top, field, t, px);
        rgb = mix(rgb, stone_lit, b.stone * params.gate_b.y) + b.light * params.gate_b.y;
    }

    // Brightest round the panel, so the edges of a wide window stay quiet.
    let vignette = 1.0 - smoothstep(0.15, 0.95, length(uv - centre));
    rgb = rgb * (0.65 + 0.35 * vignette);

    // The haze of the passage: accent, never white.
    let haze = params.hour.y;
    if haze > 0.0 {
        let fog = noise2(uv * 2.0 + vec2<f32>(t * 0.4, t * 0.4)) * haze;
        rgb = mix(rgb, AURORA, clamp(fog, 0.0, 1.0));
    }

    // The near air: motes, never over the panel.
    let off_panel = rounded_box(uv - centre, params.panel.zw, 0.02);
    var air_at = uv + pointer * 0.060 + drift * 0.02;
    air_at = centre + (air_at - centre) / params.gate_a.z;
    let motes_at = air_at * 42.0 + vec2<f32>(t * 0.07, -t * 0.12);
    let mote_seed = hash2(floor(motes_at));
    let mote_point = fract(motes_at) - vec2<f32>(mote_seed, fract(mote_seed * 7.3));
    let mote = (1.0 - smoothstep(0.015, 0.065, length(mote_point))) * step(0.985, mote_seed);
    rgb = rgb + AURORA * 0.22 * mote * step(0.0, off_panel);

    // The ring round the panel stays dark whatever is lit, so the leather's
    // edge reads at every frame of the passage.
    let ring = 1.0 - smoothstep(0.015, 0.02, abs(off_panel));
    let lum = luminance(rgb);
    let cap = mix(1.0e3, RING_CAP, ring);
    rgb = rgb * min(1.0, cap / max(lum, 1.0e-4));

    // Grain, then a triangular dither of one step of the 8-bit target, in
    // sRGB where the steps are: long dark ramps band otherwise.
    let grain = (hash2(floor(in.position.xy)) - 0.5) * 0.004;
    let dither = hash2(in.position.xy) + hash2(in.position.xy + vec2<f32>(17.3, 5.9)) - 1.0;
    let encoded = pow(max(rgb + grain, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2)) + dither / 255.0;
    rgb = pow(max(encoded, vec3<f32>(0.0)), vec3<f32>(2.2));
    return vec4<f32>(rgb, params.air.z);
}
