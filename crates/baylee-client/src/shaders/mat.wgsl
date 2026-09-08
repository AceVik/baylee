// One seat's ground: a rounded rectangle with three lanes on it, the seat's
// own colour round its edge, and a light running that edge while it is that
// seat's turn.
//
// It was a 512 × 256 RGBA image stretched over a mat about thirteen units
// wide. At this camera a card is roughly 114 physical pixels, so the mat
// wanted some two thousand texels and had five hundred — every edge on it,
// the rim included, was a soft ramp four or five pixels across. That is what
// "the battlefield lines are unsharp" was, and no amount of filtering fixes
// it: a stretched image has no idea how large it is being drawn.
//
// A distance field does. `fwidth` gives an edge exactly one pixel wide at any
// size and any camera distance, the corner radius is a length in table units
// rather than a fraction of an image, and the seat whose turn it is gets a
// light travelling round its rim — which a baked texture could not have had
// at all without a second image per frame.
//
// `baylee_client_core::tabletop::seat_mat` is the same arithmetic in Rust,
// where a test can measure that only the rim carries the seat's colour and
// that the seam sits between two lanes rather than through one. Every number
// both of them use is a `tabletop::MAT_*` constant, and
// `table::shader_tests::the_shader_and_the_generator_agree_about_the_mat`
// fails if the two drift.
//
// # WebGL2
//
// Uniforms only, no textures, no loops.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::globals

struct MatParams {
    /// The seat's colour, and `w` how brightly the mat is drawn at all — the
    /// `Mood` in one number, from `zone_brightness`.
    accent: vec4<f32>,
    /// The mat's world size, so every length below is in table units.
    size: vec2<f32>,
    /// The corner radius, in table units: `tabletop::MAT_CORNER`.
    corner: f32,
    /// How far in from the edge the coloured rim runs: `tabletop::MAT_RIM`.
    rim: f32,
    /// 1 while this is the seat whose turn it is, 0 otherwise.
    on_turn: f32,
    /// The clock the travelling light runs on: `MOVING` or `STILL`, the same
    /// two values the cards, the felt and the sky use.
    motion: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> params: MatParams;

const TAU: f32 = 6.2831855;

/// How much of white each of the three lanes is veiled with, from the lane
/// nearest the middle of the table outwards. `tabletop::MAT_LANES`.
///
/// An alpha over the felt, and therefore linear light: the note in
/// `tabletop::seat_mat` is where the quarter these were cut to is argued.
const LANE_NEAR: f32 = 0.0135;
const LANE_MID: f32 = 0.0105;
const LANE_FAR: f32 = 0.0080;

/// The hairline between two lanes, and how wide it runs as a fraction of the
/// mat's depth. `tabletop::MAT_SEAM`, `tabletop::MAT_SEAM_WIDTH`.
const SEAM: f32 = 0.036;
const SEAM_W: f32 = 0.014;

/// How much of white the rim carries at its brightest.
/// `tabletop::MAT_RIM_LIGHT`.
const RIM_LIGHT: f32 = 0.62;

/// How hard the rim's opacity falls off, and how hard its hue does.
/// `tabletop::MAT_RIM_FALL`, `tabletop::MAT_HUE_FALL`.
///
/// Two different exponents on purpose. Reusing the opacity's curve for the
/// colour is the tidy version and renders a washed-out rim — 1.3 is steep, so
/// the accent only approaches full strength in the last fraction of the rim,
/// where the edge is feathering it away as well. A shallower exponent spreads
/// the hue across the whole rim while the opacity keeps its own edge, and the
/// seat colours separate.
const RIM_FALL: f32 = 1.3;
const HUE_FALL: f32 = 0.55;

/// The light that says whose turn it is: how bright it burns, how long it
/// takes to travel once round the mat, and how much of the rim it covers at a
/// time as a fraction of the way round.
///
/// A short comet rather than a border that brightens and dims together. A
/// whole rim pulsing is the shape every interface uses for *something is
/// wrong*, and this says only "it is your turn" — a thing that is true for
/// most of a game and must therefore be able to sit there without nagging.
/// It travels, which reads at the edge of vision, and at any one moment four
/// fifths of the rim is exactly the rim.
const TURN_LIGHT: f32 = 0.55;
const TURN_SECONDS: f32 = 3.6;
const TURN_ARC: f32 = 0.22;

/// How much of the comet is left where the rim has already faded out.
///
/// Not zero: `falloff` is 1 at the very edge and 0 an inch in, so tying the
/// light to it alone puts the whole comet in the outermost pixel, where the
/// coverage ramp is also eating it. This lifts it far enough inboard to be a
/// light on the border rather than a light on the outline.
const TURN_REACH: f32 = 0.45;

/// A rounded rectangle's signed distance: negative inside, positive outside.
fn sd_round_box(p: vec2<f32>, half: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - half + vec2<f32>(r);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let half = params.size * 0.5;
    // The mesh is a `Rectangle` and its uv runs corner to corner, so this is
    // the mat's own space in table units. Deliberately *not* the world
    // position: that would have to be un-rotated by the seat's facing first,
    // and a seat's facing is a matrix this shader is not given.
    let p = (in.uv - vec2<f32>(0.5)) * params.size;
    let d = sd_round_box(p, half, min(params.corner, min(half.x, half.y)));

    // One pixel of edge, whatever the mat's size and wherever the camera is.
    let aa = max(fwidth(d), 1e-5);
    let coverage = clamp(0.5 - d / aa, 0.0, 1.0);
    if coverage <= 0.0 {
        discard;
    }

    // Three lanes down the mat's depth, brightest at the front where the
    // creatures stand, and a hairline between them so the rows separate
    // without a border drawn round each one. `uv.y = 0` is the edge nearest
    // the middle of the table, which is the same end `seat_mat` paints
    // `MAT_LANES[0]` at.
    let v = in.uv.y;
    var lane = LANE_FAR;
    if v < 1.0 / 3.0 {
        lane = LANE_NEAR;
    } else if v < 2.0 / 3.0 {
        lane = LANE_MID;
    }
    let to_seam = min(abs(v - 1.0 / 3.0), abs(v - 2.0 / 3.0));
    let seam = clamp(1.0 - to_seam / SEAM_W, 0.0, 1.0) * SEAM;

    // The rim: the one part meant to be read from across the table, since it
    // is what carries the seat's colour.
    let inset = -d;
    let falloff = clamp(1.0 - inset / max(params.rim, 1e-4), 0.0, 1.0);
    let border = pow(falloff, RIM_FALL);
    let hue = pow(falloff, HUE_FALL);

    // And the light that says whose turn it is, on the rim and nowhere else.
    // It is *added* to the rim rather than replacing it, so a seat's own
    // colour still says which seat this is while it runs.
    let turn = atan2(p.y * half.x, p.x * half.y) / TAU + 0.5;
    let lead = fract(globals.time * params.motion / TURN_SECONDS);
    let comet = pow(clamp(1.0 - fract(turn - lead) / TURN_ARC, 0.0, 1.0), 2.0);
    let reach = smoothstep(0.0, TURN_REACH, falloff);
    let running = params.on_turn * comet * reach * TURN_LIGHT;

    // Brightness scales the alpha, not the colour, which is the one thing
    // that changed meaning when the mat stopped being a white texture under a
    // tinted material. It is the reading `zone_brightness` already describes:
    // a seat that has lost fades *into* the felt, rather than drawing a dark
    // grey rim over it and staying just as visible as everyone else.
    let value = (lane + seam + border * RIM_LIGHT + running) * params.accent.w;
    let colour = mix(vec3<f32>(1.0), params.accent.rgb, hue);
    return vec4<f32>(colour, clamp(value, 0.0, 1.0) * coverage);
}
