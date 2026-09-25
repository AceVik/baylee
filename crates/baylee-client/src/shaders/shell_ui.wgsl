// The shells round a permanent on its preview (the PM, 25.09): the table
// shader's twin, as `plate_ui.wgsl` is `plate.wgsl`'s. A card held up to the
// light wears what it wears on the felt: indestructible's darksteel rim,
// hexproof's or shroud's dome, defender's wall, summoning sickness's wave.
//
// The preview is the card laid flat on the screen, so there is no camera to
// follow a ray from. Each shell is drawn as it stands, seen from straight
// over the card, which is what lines it up with the flat print, and lit as
// a duel's seat sees a card of its own lit ([`LEAN`]). It always stands: the
// preview has air round it, so nothing lies down as a ring, and there is no
// felt for a shadow to fall on. What both shaders draw with is
// `shell_common.wgsl`'s, so the steel, the glass and the brick are the
// table's.
//
// Nothing but a dome and the wave is drawn over the print. From straight
// over the card a point's ray meets the face right under it, so the mask is
// the card's own outline (`clear`), and every colour returned but the dome's
// and the wave's carries it; a test reads every `return` below to make sure
// it still does.

#import bevy_render::globals::Globals
#import bevy_ui::ui_vertex_output::UiVertexOutput
#import "embedded://baylee_client/shaders/card_common.wgsl"::{perimeter, CARD_ASPECT}
#import "embedded://baylee_client/shaders/shell_common.wgsl"::{SHELL_DOME, SHELL_WALL, SHELL_WAVE, WAVE_PERIOD, MOONLIGHT, CARD_HALF_W, CARD_HALF_H, CARD_ROUND, MASK_FEATHER, RIM_DROP, WALL_COURSE, WALL_HEIGHT, KEY, card_sdf, card_out, rim_steel, dome_glass, brick, wave_along, wave_glow}

struct ShellUiParams {
    /// A dome's colour, linear; the steel and the wall ignore it.
    tint: vec4<f32>,
    /// The node, `[x0, y0, x1, y1]` in card widths from the card's top-left
    /// corner, `y` down it: `shellmat::PREVIEW_QUAD`.
    quad: vec4<f32>,
    /// `shellmat::ShellKind`.
    kind: u32,
    /// The clock every animated term runs on: 1 normally, 0 for
    /// `Preferences::reduce_motion`.
    motion: f32,
    /// A dome's row: its foot past the card's edge and its crown over the
    /// face (`shellmat::DomeRow`).
    dome: vec2<f32>,
}

@group(0) @binding(1) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> params: ShellUiParams;

/// The rim's foot past the card's edge. `shellmat::RIM_MARGIN`.
const RIM_MARGIN: f32 = 0.06;
/// How far in from the card's edge a dome's crown stands, how much rounder
/// a ring of it grows for each unit inside the edge, and the smallest round
/// it keeps. `shellmat::DOME_CROWN`, `DOME_ROUNDING` and `CAP_ROUND`.
const DOME_CROWN: f32 = 0.47;
const DOME_ROUNDING: f32 = 1.5;
const CAP_ROUND: f32 = 0.03;
/// The wall (`shellmat::wall_arc`, `wall_mesh`): how far past the card's top
/// edge its foot stands, how far its middle bows out, how thick it is, how
/// far its face towards the card leans back, how far past the card's sides
/// it runs, and how many merlons it has.
const WALL_NEAR: f32 = 0.12;
const WALL_BULGE: f32 = 0.04;
const WALL_THICK: f32 = 0.04;
const WALL_BATTER: f32 = 0.06;
const WALL_OVERHANG: f32 = 0.06;
const WALL_MERLONS: f32 = 5.0;
/// How a duel's seat sees a card of its own: from this far off vertical, as
/// a tangent, towards the card's foot. `table::DUEL_LEAN`.
const LEAN: f32 = 0.62;

/// How round a dome's ring standing `d` past the card's edge is at its
/// corners: the card's round outside the edge, rounder the further in, until
/// its ends are half circles. `shellmat::outline`.
fn ring_round(d: f32) -> f32 {
    return max(CARD_ROUND + d, min(CARD_ROUND - DOME_ROUNDING * d, CARD_HALF_W + d));
}

/// Signed distance from `p` to a dome's ring standing `d` past the card's
/// edge, seen from above.
fn ring_sdf(p: vec2<f32>, d: f32) -> f32 {
    let r = ring_round(d);
    let q = abs(p) - (vec2<f32>(CARD_HALF_W + d, CARD_HALF_H + d) - vec2<f32>(r));
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

/// Which way is out from a dome's ring standing `d` past the card's edge,
/// at `p`, in the card's plane.
fn ring_out(p: vec2<f32>, d: f32) -> vec2<f32> {
    let r = ring_round(d);
    let q = abs(p) - (vec2<f32>(CARD_HALF_W + d, CARD_HALF_H + d) - vec2<f32>(r));
    return sign(p) * normalize(max(q, vec2<f32>(1e-5)));
}

/// Which of a dome's rings passes over `p` seen from above: how far past the
/// card's edge it stands. Outside the edge that is `card_sdf`; inside, a ring
/// is rounder than the card, so it is found in a few steps from there.
fn dome_ring(p: vec2<f32>) -> f32 {
    var d = card_sdf(p);
    for (var i = 0; i < 4; i++) {
        d += ring_sdf(p, d);
    }
    return d;
}

/// The middle line of the wall's top at `x` across the card: where it is
/// and which way is out, towards the table's middle. `shellmat::wall_arc`.
fn wall_line(x: f32) -> vec4<f32> {
    let chord = 2.0 * CARD_HALF_W + 2.0 * WALL_OVERHANG;
    let across = 2.0 * x / chord;
    let near = CARD_HALF_H + WALL_NEAR + WALL_BATTER + 0.5 * WALL_THICK;
    let y = near + WALL_BULGE * (1.0 - across * across);
    let slope = -8.0 * WALL_BULGE * x / (chord * chord);
    return vec4<f32>(x, y, normalize(vec2<f32>(-slope, 1.0)));
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    // The point, in card widths from the card's top-left corner, `y` down
    // it; then in the shell's own space. The preview's card is
    // 1/`CARD_ASPECT` card widths tall and the table's `2 * CARD_HALF_H`,
    // which differ in the fourth place: the print's outline is laid where
    // the preview draws it.
    let q = mix(params.quad.xy, params.quad.zw, in.uv);
    let p = vec2<f32>(q.x - CARD_HALF_W, CARD_HALF_H - q.y * (2.0 * CARD_HALF_H * CARD_ASPECT));
    // The card's own UV, running on past its edges: the steel's band of
    // light goes round by it.
    let uv = vec2<f32>(q.x, q.y * CARD_ASPECT);
    // Taken here, in uniform control flow, before anything branches.
    let aa = max(fwidth(p.x), 1e-4);
    let d = card_sdf(p);
    // Nothing of the rim or the wall over the print.
    let clear = smoothstep(0.0, MASK_FEATHER, d);

    let m = params.motion;
    // Towards the eye, and the light, in the card's own space: a duel's
    // seat looking at a card of its own, whose top points away from it.
    let v = normalize(vec3<f32>(0.0, -LEAN, 1.0));
    let key = vec3<f32>(KEY.x, -KEY.z, KEY.y);
    // One band of light round the card, a lap every seven seconds, and a
    // dome's slow breath, both at their means with motion off.
    let lap = 1.0 - 2.0 * abs(fract(perimeter(uv) - 0.15 * globals.time * m) - 0.5);
    let band = mix(1.0 / 7.0, pow(lap, 6.0), m);
    let breath = 1.05 + 0.05 * m * sin(globals.time * 1.047);

    if params.kind == SHELL_WAVE {
        // The table's wave over the print, seen from straight over it, at
        // the preview's own moment: its sheet is the card's face, which is
        // where `p` already lies.
        let glow = wave_glow(p, wave_along(globals.time * m / WAVE_PERIOD, m), v, key);
        return vec4<f32>(MOONLIGHT, glow);
    }
    if params.kind == SHELL_DOME {
        // Standing full: a quarter ellipse from its crown, a ridge down the
        // card's middle, to its foot on the felt past the card's edge
        // (`shellmat::dome_profile` at `DOME_STEPS[0]`). From above, the
        // ring over the point says how far out it is and which way its
        // glass faces.
        let margin = params.dome.x;
        let base = select(0.0, -RIM_DROP, margin > 0.0);
        let run = margin + DOME_CROWN;
        let rise = params.dome.y - base;
        let ring = dome_ring(p);
        let out = clamp((ring + DOME_CROWN) / run, 0.0, 1.0);
        let up = sqrt(max(1.0 - out * out, 0.0));
        let n = normalize(vec3<f32>(ring_out(p, ring) * rise * out, run * up));
        let lit = dome_glass(out, n, v, key, params.tint.rgb, breath);
        let colour = lit.rgb;
        let glass = lit.a * (1.0 - smoothstep(margin - aa, margin + aa, d));
        return vec4<f32>(colour, glass);
    }
    if params.kind == SHELL_WALL {
        // Where the wall's middle line passes the point: found at its `x`,
        // then once more along the normal there, the arc being shallow.
        var line = wall_line(p.x);
        var across = dot(p - line.xy, line.zw);
        let x = p.x - across * line.z;
        line = wall_line(x);
        across = dot(p - line.xy, line.zw);
        let chord = 2.0 * CARD_HALF_W + 2.0 * WALL_OVERHANG;
        let along = x + 0.5 * chord;
        let half = 0.5 * WALL_THICK;
        let crenels = 2.0 * WALL_COURSE;
        // From above: the face towards the card, leaning back from its foot
        // to the crenels' height; then the tops, a merlon's at the wall's
        // height and the crenels' between them. The face towards the table's
        // middle stands upright and shows nothing.
        let face = across < -half;
        let merlon = fract(along / chord * (2.0 * WALL_MERLONS - 1.0) * 0.5) < 0.5;
        let top_h = select(crenels, WALL_HEIGHT, merlon);
        let face_h = crenels * clamp((across + half + WALL_BATTER) / WALL_BATTER, 0.0, 1.0);
        let h = select(top_h, face_h, face);
        let out = vec3<f32>(line.zw, 0.0);
        let local_n = select(
            vec3<f32>(0.0, 0.0, 1.0),
            normalize(vec3<f32>(0.0, 0.0, WALL_BATTER) - out * crenels),
            face,
        );
        // `brick` reads its normal in the world, `y` up.
        let n = vec3<f32>(local_n.x, local_n.z, -local_n.y);
        let span = smoothstep(-aa, aa, along) * (1.0 - smoothstep(chord - aa, chord + aa, along));
        let band_in = smoothstep(-half - WALL_BATTER - aa, -half - WALL_BATTER + aa, across)
            * (1.0 - smoothstep(half - aa, half + aa, across));
        return vec4<f32>(brick(along, h, n, aa), span * band_in * clear);
    }
    // The rim: from its top edge on the card's edge down its slope to its
    // foot on the felt, `RIM_MARGIN` out.
    let down = clamp(d / RIM_MARGIN, 0.0, 1.0);
    let rim = 1.0 - smoothstep(RIM_MARGIN - aa, RIM_MARGIN + aa, d);
    return vec4<f32>(rim_steel(card_out(p), down, v, key, band), rim * clear);
}
