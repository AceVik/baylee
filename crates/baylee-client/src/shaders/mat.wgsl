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
// `baylee_client_core::tabletop::seat_mat` is the base-layout reference:
// only the rim carries seat colour, and seams sit between the same lanes.
// The shader adds recessed tooling and resolves grain to the pixel footprint.
// `table::camera_tests` checks the shared `tabletop::MAT_*` constants.
//
// # The browser's budget
//
// Uniforms only, no textures, no loops — the WebGL2 envelope, kept although
// the browser build renders through WebGPU now. See `cardmat`'s header.

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
    /// 1 while this is the seat the engine is waiting on an answer from.
    awaited: f32,
    /// The clock the travelling light runs on: `MOVING` or `STILL`, the same
    /// two values the cards, the felt and the sky use.
    motion: f32,
    /// 1 when this seat's shelf is on the mat's *outer* edge, 0 when it is on
    /// the centre-facing one: `layout::LEDGE_IS_OUTER`.
    ///
    /// A seat drawn across the table has its board upside-down from here, so
    /// its shelf goes at the far end of the mat and its bar is still above
    /// its creatures on the screen somebody is looking at. The lanes do not
    /// turn round with it — where a card stands is the seat's own business —
    /// which is why this cannot be a flipped uv and has to be a flag.
    ledge_outer: f32,
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

/// The shelf at the mat's centre-facing edge that the seat's bar is written
/// on: how much of white it carries, and where it ends as a fraction of the
/// mat's depth. `tabletop::MAT_LEDGE_VALUE`, `tabletop::LEDGE_FRAC`.
///
/// Dimmer than the quietest lane, because the ink on it is meant to be the
/// brightest thing on a seat's ground.
const LANE_LEDGE: f32 = 0.0060;
const LEDGE_FRAC: f32 = 0.220002328;

/// One lane and one end of the printed border, as fractions of the same
/// depth. `tabletop::LANE_FRAC`, `tabletop::MARGIN_FRAC`.
///
/// Every fraction here is over the mat that is **drawn**, which is
/// `tabletop::MAT_MARGIN` deeper than the playing extent at each end. They
/// were over the playing extent, which stretched all four bands by 18.5% and
/// is why the shelf and the bar written on it disagreed by 0.46 units.
/// `LEDGE_FRAC + 3·LANE_FRAC + MARGIN_FRAC` is 1: the shelf, the three rows
/// and the border beyond the last of them are the whole mat.
const LANE_FRAC: f32 = 0.200929782;
const MARGIN_FRAC: f32 = 0.078065342;

/// The hairline between two lanes, and how wide it runs as a fraction of the
/// mat's depth. `tabletop::MAT_SEAM`, `tabletop::MAT_SEAM_WIDTH`.
const COMBAT_FRAC: f32 = 0.099142984;

const SEAM: f32 = 0.036;
const SEAM_W: f32 = 0.0075;

/// How much brighter the ledge's own seam is than a seam between two lanes.
/// `tabletop::MAT_LEDGE_SEAM`.
const LEDGE_SEAM: f32 = 1.5;

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

/// The light that says whose turn it is: how bright it burns, how long a
/// swell takes to travel once round the mat, and how much of the rim that
/// swell covers as a fraction of the way round.
///
/// It was a short comet at 3.6 seconds a lap, and the owner's word on it was
/// that it is too fast and should read as a breath rather than a chase. So
/// there are two clocks now and they are deliberately not in step: a swell
/// that travels the rim once every eleven seconds, and a breath the whole rim
/// takes together every seven. Two periods with no common multiple worth
/// noticing means the rim never repeats a pose, which is what makes slow
/// movement read as alive rather than as a loop.
///
/// The arc is more than half the way round now. A comet is a chase and asks
/// to be followed; a swell this wide is a tide, and a player reading a card
/// never catches it happening.
const TURN_LIGHT: f32 = 0.72;
const TURN_SECONDS: f32 = 11.0;
const TURN_ARC: f32 = 0.55;
const BREATH_SECONDS: f32 = 7.0;

/// How much light the rim holds when the swell is elsewhere, and how far the
/// breath takes it down at the bottom of one.
///
/// Neither is zero, and that is the difference between a light that *travels*
/// and one that **glows**: the whole rim of the seat on turn is lit the whole
/// time, and the swell and the breath move over a light that is already
/// there. A rim that went dark between passes would be blinking.
const TURN_BASE: f32 = 0.38;
const BREATH_LOW: f32 = 0.55;

/// How much of the light is left where the rim has already faded out.
///
/// Not zero: `falloff` is 1 at the very edge and 0 an inch in, so tying the
/// light to it alone puts the whole of it in the outermost pixel, where the
/// coverage ramp is also eating it. Raised from 0.45, which is the rest of
/// "und auch glühen": the same light spread across three times the width of
/// border is a glow, and in one pixel it is an outline.
const TURN_REACH: f32 = 0.85;

// Brushed champagne in the frame, not in the playing lanes. Linear colour.
const FRAME_INK: vec3<f32> = vec3<f32>(0.31, 0.245, 0.15);

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
    // The information band is outside the battlefield. Crop it before
    // constructing the rim so no outline encloses names, life or phases.
    // Lane coordinates remain unchanged: cards keep all three existing rows.
    let band = params.size.y * LEDGE_FRAC;
    let sign = select(1.0, -1.0, params.ledge_outer > 0.5);
    let field_half = vec2<f32>(half.x, half.y - band * 0.5);
    let field_p = p - vec2<f32>(0.0, band * sign * 0.5);
    let d = sd_round_box(field_p, field_half, min(params.corner, min(field_half.x, field_half.y)));

    // One pixel of edge, whatever the mat's size and wherever the camera is.
    let aa = max(fwidth(d), 1e-5);
    let footprint = fwidth(p);
    let coverage = clamp(0.5 - d / aa, 0.0, 1.0);
    if coverage <= 0.0 {
        discard;
    }

    // The seat's ledge and three lanes, brightest at the front where the
    // creatures stand, and a hairline between them so the rows separate
    // without a border drawn round each one. `uv.y = 0` is the edge nearest
    // the middle of the table, which is the same end `seat_mat` measures
    // from.
    //
    // The lanes always run from that end outwards; only the shelf moves. So
    // the block of three starts at `first` — one border in, and one shelf
    // further when the shelf is standing at this end — while `from_shelf` is
    // the depth read from whichever end the shelf took, which answers both
    // "is this the shelf" and "how far is the fence" without a second case.
    //
    // The printed border is not a band of its own: at the shelf's end it is
    // the shelf, and past the last lane it is that lane running out to the
    // rim, which is what `LANE_FAR` being the default leaves it as.
    let v = in.uv.y;
    let outer = params.ledge_outer > 0.5;
    let first = MARGIN_FRAC + select(LEDGE_FRAC - MARGIN_FRAC, 0.0, outer);
    let from_shelf = select(v, 1.0 - v, outer);
    let a = first + COMBAT_FRAC + LANE_FRAC;
    let b = first + COMBAT_FRAC + LANE_FRAC * 2.0;
    var lane = LANE_FAR;
    if from_shelf < LEDGE_FRAC {
        lane = LANE_LEDGE;
    } else if v < a {
        lane = LANE_NEAR;
    } else if v < b {
        lane = LANE_MID;
    }
    let to_seam = min(abs(v - a), abs(v - b));
    let lane_seam = clamp(1.0 - to_seam / SEAM_W, 0.0, 1.0) * SEAM;
    // The ledge's own boundary is a seam too, and a brighter one: it is where
    // a seat's ground stops being a place cards stand on and becomes a shelf
    // they are described on.
    // Dividers end in engraved shoulders rather than crossing the whole
    // field like spreadsheet rules. The lane boundaries do not move.
    let seam_end = smoothstep(0.015, 0.065, in.uv.x) * (1.0 - smoothstep(0.935, 0.985, in.uv.x));
    let seam = lane_seam * seam_end;

    // The rim: the one part meant to be read from across the table, since it
    // is what carries the seat's colour.
    let inset = -d;
    let falloff = clamp(1.0 - inset / max(params.rim, 1e-4), 0.0, 1.0);
    let border = pow(falloff, RIM_FALL);
    let hue = pow(falloff, HUE_FALL);

    // And the light that says whose turn it is, on the rim and nowhere else.
    // It is *added* to the rim rather than replacing it, so a seat's own
    // colour still says which seat this is while it runs.
    //
    // Three terms, and each answers a different half of "wie ein Atemfluss":
    // `swell` is the flow, a wide soft tide going round once every eleven
    // seconds; `breath` is the whole rim rising and falling together on its
    // own seven-second clock; and `TURN_BASE` is the light both of them move
    // over, which is what makes it a glow rather than a signal.
    let turn = atan2(field_p.y * field_half.x, field_p.x * field_half.y) / TAU + 0.5;
    let lead = fract(globals.time * params.motion / TURN_SECONDS);
    // A circular distance gives the swell a soft front as well as a tail:
    // no jump from zero to full light when the travelling phase wraps.
    let distance = abs(fract(turn - lead + 0.5) - 0.5);
    let swell = 1.0 - smoothstep(0.0, TURN_ARC * 0.5, distance);
    let breath = mix(
        BREATH_LOW,
        1.0,
        0.5 + 0.5 * sin(globals.time * params.motion * TAU / BREATH_SECONDS)
    );
    let reach = smoothstep(0.0, TURN_REACH, falloff);
    let flow = TURN_BASE + (1.0 - TURN_BASE) * swell;
    let halo = exp(-inset * 7.5);
    let running = params.on_turn * (0.58 + 0.42 * flow * breath) * halo * 0.95
        + params.awaited * (0.30 + 0.10 * breath) * halo;


    // Brightness scales the alpha, not the colour, which is the one thing
    // that changed meaning when the mat stopped being a white texture under a
    // tinted material. It is the reading `zone_brightness` already describes:
    // a seat that has lost fades *into* the felt, rather than drawing a dark
    // grey rim over it and staying just as visible as everyone else.
    // A recessed double frame, with short corner shoulders. This detail
    // belongs to the object, while the outer light still belongs to the seat.
    let inner = 1.0 - smoothstep(0.010, 0.010 + aa, abs(inset - 0.14));
    let shoulder = smoothstep(0.64, 0.87, abs(p.x) / half.x);
    let tooling = inner * (0.12 + shoulder * 0.55);
    let grain = fract(sin(dot(floor(p * 95.0), vec2<f32>(12.9898, 78.233))) * 43758.5453);
    let resolved = 1.0 - smoothstep(0.2, 0.6, max(footprint.x, footprint.y) * 95.0);
    let surface = lane * (0.86 + 0.14 * cos(p.y * 1.4)) + (grain - 0.5) * 0.002 * resolved;
    let value = (surface + seam + border * RIM_LIGHT + running + tooling * 0.085) * params.accent.w;
    let base_colour = mix(vec3<f32>(1.0), params.accent.rgb, hue);
    let signal_colour = mix(params.accent.rgb, vec3<f32>(0.25, 0.72, 0.82), params.awaited * 0.20);
    let colour = mix(mix(base_colour, signal_colour, clamp(running * 12.0, 0.0, 1.0)), FRAME_INK, tooling * 0.5);
    return vec4<f32>(colour, clamp(value, 0.0, 1.0) * coverage);
}
