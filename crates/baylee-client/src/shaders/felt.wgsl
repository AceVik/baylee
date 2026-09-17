// The table the game is played on: midnight mineral cloth inside a machined
// bronze rail, with engraved inlays and a shadowed apron below it.
//
// No texture at all. The slab is about thirty-five units across and a card is
// roughly 114 physical pixels at this camera, so drawing the cloth sharply
// would want some four thousand texels — and measured in a debug build,
// generating 2048 already costs 1.6 seconds every time the table is re-cut.
// Arithmetic has no resolution.
//
// `baylee_client_core::tabletop::felt` remains the base-cloth reference. The
// shader adds the machined frame and its light; the CPU bounds the cloth's
// texture and brightness. `table::camera_tests` checks the shared palette.
//
// # What is drawn where
//
// One rounded-rectangle field decides all of it. The mesh is already cut to
// that shape, so the boundary here is not a cut-out: it is where the felt
// stops and the rail begins, one rail width inside the mesh's own edge. The
// wall of the slab is the same material, told apart by its normal — the top
// face points up and nothing else does.
//
// The phase lamp that used to run down a channel of resin runs round the
// **rail** instead. It says the same thing it always did — where in the turn
// the game is, entering at the active seat's own shore — and it says it on
// the one part of the table no card ever lies on.
//
// # WebGL2
//
// The browser build targets WebGL2: uniforms only, no storage buffers, no
// texture arrays, every loop bound at compile time. The one loop below counts
// to four literally. `globals.time` comes from the view bind group, so the
// rail runs without the CPU touching a material asset per frame.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::globals

struct FeltParams {
    /// The phase lamp: `rgb` its colour, `a` how much of it there is.
    wash: vec4<f32>,
    /// Where the light enters the rail — `xy` a point on the active seat's
    /// own edge in table space, `zw` the direction it travels from there.
    source: vec4<f32>,
    /// The light the room is in: `rgb` a multiplier on the table's own
    /// colour, `a` how much of it arrives. `tabletop`'s colours are the
    /// cloth at `a = 0`.
    ///
    /// It reaches the felt, the rail and the apron and stops at the phase
    /// lamp, which is a light the *table* emits and carries a meaning of its
    /// own — a wash that went blue after sunset would be saying something
    /// about the turn that was not true.
    ambient: vec4<f32>,
    /// The weather in the air over the table: `rgb` a second multiplier on
    /// the table's own colour, `w` unused. `(1, 1, 1)` is still air.
    ///
    /// Its own field and not folded into `ambient`, because the sky's light
    /// arrives at a strength that depends on the hour and the weather's does
    /// not — multiplied together, a forest would stop being green at noon.
    weather: vec4<f32>,
    /// How hard the first four flames of the firewheel burn, 0 to 1: white,
    /// blue, black, red. `baylee_client_core::firewheel` is normative.
    flames: vec4<f32>,
    /// `x` green's strength; `yz` **screen-up in table space**, the one
    /// direction every flame rises along; `w` spare.
    ///
    /// Screen-up is a uniform and not a per-flame radial direction, which is
    /// the difference between five candles seen from one chair and a sun
    /// glyph. It is the local seat's own inward direction, so it stays right
    /// at four seats and at eight.
    flames_tail: vec4<f32>,
    /// The slab's world size, which is what turns a world position into a
    /// point in the fields below.
    span: vec2<f32>,
    /// The corner radius the mesh was cut with, so the rail follows the same
    /// racetrack the timber does.
    corner: f32,
    /// How wide the padded rail runs, in table units.
    rail: f32,
    /// The clock the lamp runs on: 1 normally, 0 for reduce-motion.
    motion: f32,
    /// How hard the lamp burns at full energy. Above 1 on purpose, so combat
    /// blooms and the cards — which are unlit — do not.
    gain: f32,
    /// How thick the slab is, so the apron can be shaded down its height.
    thickness: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> params: FeltParams;

// The baize, and the rail around it. Display-referred, like every colour this
// project writes down, and therefore run through `to_linear` before use.
//
// That conversion is not a detail: an earlier version of this table added its
// numbers straight into a linear render target, and a surface meant to sit at
// 0.22 measured 0.45 on screen.
const FELT_DEEP: vec3<f32> = vec3<f32>(0.043, 0.072, 0.080);
const FELT_CLOTH: vec3<f32> = vec3<f32>(0.120, 0.188, 0.204);
const FELT_WORN: vec3<f32> = vec3<f32>(0.165, 0.245, 0.258);
const RAIL_HIDE: vec3<f32> = vec3<f32>(0.100, 0.084, 0.064);
const RAIL_LIP: vec3<f32> = vec3<f32>(0.300, 0.244, 0.157);
const APRON: vec3<f32> = vec3<f32>(0.040, 0.035, 0.029);

// Fixed locations, not cycling hues: these are ornament, never priority.
const INLAY_WHITE: vec3<f32> = vec3<f32>(0.94, 0.91, 0.80);
const INLAY_BLUE: vec3<f32> = vec3<f32>(0.29, 0.55, 0.83);
const INLAY_BLACK: vec3<f32> = vec3<f32>(0.30, 0.27, 0.34);
const INLAY_RED: vec3<f32> = vec3<f32>(0.83, 0.36, 0.28);
const INLAY_GREEN: vec3<f32> = vec3<f32>(0.36, 0.66, 0.42);
const ENGRAVING: vec3<f32> = vec3<f32>(0.52, 0.44, 0.31);
/// Worn gold, for the firewheel's two rings.
const GILT: vec3<f32> = vec3<f32>(0.62, 0.50, 0.26);

const TAU: f32 = 6.2831855;

/// The table under whatever sky is behind it.
///
/// A multiply, not a mix towards a colour: light is what a surface reflects,
/// so a warm sky over green cloth has to be able to lift the red end without
/// touching the green, and a mix would drag every channel towards the light's
/// own hue and turn the baize grey at both ends of the day.
fn under_sky(linear: vec3<f32>) -> vec3<f32> {
    return mix(linear, linear * params.ambient.rgb, params.ambient.a);
}

/// The same cloth with weather in the air over it.
///
/// `baylee_client_core::atmosphere` builds this so that its Rec.709 luma is
/// exactly 1 however many kinds of land are on the table, which is the one
/// property that matters: the eye reads a change in *lightness* on the cloth
/// as a change in the room, and a change in the room behind a card is exactly
/// what would make the card harder to judge. Hue may move; brightness may not.
///
/// Unscaled, unlike `under_sky` — the strength is already in the number,
/// because how much of it there is was decided by how many forests are on the
/// table and by what the player asked for, and neither of those is the
/// shader's business.
fn under_weather(linear: vec3<f32>) -> vec3<f32> {
    return linear * params.weather.rgb;
}

/// The lamp over the table: how far its pool reaches as a fraction of the
/// slab's half-span, and how much light is left outside it.
///
/// A card room is lit from over the table and nowhere else, and this is the
/// whole of that. It **darkens the ends** rather than brightening the middle,
/// which is the only way to have it at all here: the cloth is already as
/// bright as it is allowed to be (`the_felt_is_dark_enough_to_read_cards_
/// against` bounds it from both sides), so a lamp that lifted the centre
/// would be a table competing with the cards on it. Falling away at the edges
/// costs nothing and says the same thing.
///
/// Like [`under_sky`] it is a multiply on the table's own colour, and for the
/// same reason: there is no light in this scene and there cannot be one,
/// because scene lighting on card art makes colour identity unreadable. The
/// table carries its own lamp; the cards standing on it do not.
const SPOT_REACH: f32 = 0.95;
const SPOT_FLOOR: f32 = 0.62;

/// The same surface under the lamp hanging over the table.
///
/// Elliptical rather than round — the slab is a good deal wider than it is
/// deep, and a circular pool on it puts the near and far seats in the light
/// while the two at the ends sit in the dark.
fn under_lamp(linear: vec3<f32>, table: vec2<f32>) -> vec3<f32> {
    let reach = max(params.span * 0.5 * SPOT_REACH, vec2<f32>(1e-3));
    let r = length(table / reach);
    return linear * mix(1.0, SPOT_FLOOR, smoothstep(0.0, 1.0, r));
}

/// Threads per table unit. A card is one unit wide, so this is how many
/// threads cross a card. Fine enough to read as cloth rather than a checker
/// grid; the pixel footprint fades it away before it can alias.
const WEAVE: f32 = 11.0;

/// How far in from the cloth's edge the crest of the roll catches the light,
/// in table units, and how much of a lift it gets there.
///
/// This is the table turned inside out, and the inversion is the point. The
/// cloth used to fall into `FELT_DEEP` over 1.8 units as it reached the rail,
/// which is a *well*: the surface reads as sunk below the frame around it,
/// like a snooker table or a tray. An altar is the other way round — the top
/// is the highest thing there is and its edge is rounded over and away, so
/// the light sits **on** the boundary rather than dying at it. There is no
/// light in this scene to do that, so it is painted, which is the same
/// argument the cards' contact shadows make.
const ROLL: f32 = 0.9;
const ROLL_LIGHT: f32 = 0.030;

/// The light left in the rail when no step is calling for any.
const RESTING: f32 = 0.010;

/// What colour that resting light is — and it is deliberately *not* the
/// step's. The lamp is what says which step it is, and at rest there is
/// barely a lamp, so at rest the leather is lit by little more than the room.
const RAIL_LIGHT: vec3<f32> = vec3<f32>(0.90, 0.82, 0.70);

/// Turns of the travelling pulse around the whole rail, and how fast it goes.
///
/// Slow, and shallow. A player reading a card must never catch the table
/// moving out of the corner of their eye; what this is for is that a table
/// which never moves at all reads as a table that has stopped working.
const PULSE_TURNS: f32 = 3.0;
const PULSE_SPEED: f32 = 0.55;

/// sRGB to linear, componentwise.
fn to_linear(c: vec3<f32>) -> vec3<f32> {
    let hi = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    let lo = c / 12.92;
    return select(lo, hi, c > vec3<f32>(0.04045));
}

fn hash2(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash2(i);
    let b = hash2(i + vec2<f32>(1.0, 0.0));
    let c = hash2(i + vec2<f32>(0.0, 1.0));
    let d = hash2(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

/// Four octaves, normalised back to 0..1 so callers can centre it on a half.
fn fbm2(p: vec2<f32>) -> f32 {
    var sum = 0.0;
    var amplitude = 0.5;
    var total = 0.0;
    var at = p;
    for (var i = 0; i < 4; i = i + 1) {
        sum = sum + amplitude * vnoise(at);
        total = total + amplitude;
        at = at * 2.03;
        amplitude = amplitude * 0.5;
    }
    return sum / total;
}

/// The same four octaves, with the ones finer than a pixel faded out.
///
/// `step` is how much of the noise's own domain one pixel covers, so an
/// octave whose features have fallen under the footprint mixes towards the
/// mean instead of crawling. It is `baize_at`'s `resolved` idiom taken one
/// level down: the flames run at seven features per table unit, which is six
/// logical pixels at a duel and four on an eight-seat ring, and the fourth
/// octave there is one pixel across and would sparkle along every tip.
fn fbm2_faded(p: vec2<f32>, step: f32) -> f32 {
    var sum = 0.0;
    var amplitude = 0.5;
    var total = 0.0;
    var at = p;
    var scale = step;
    for (var i = 0; i < 4; i = i + 1) {
        let resolved = 1.0 - smoothstep(0.2, 0.5, scale);
        sum = sum + amplitude * mix(0.5, vnoise(at), resolved);
        total = total + amplitude;
        at = at * 2.03;
        scale = scale * 2.03;
        amplitude = amplitude * 0.5;
    }
    return sum / total;
}

fn hash1(x: f32) -> f32 {
    return fract(sin(x * 127.1) * 43758.5453);
}

/// Three octaves of value noise along a line — a rhythm, not a surface.
///
/// The flames gutter on this rather than on a sine, which is the whole of
/// what "variabler Flackerrythmus" asks for: a sine is a rhythm a viewer
/// learns in two cycles, and noise is one they never do.
fn fbm1(x: f32) -> f32 {
    var sum = 0.0;
    var amplitude = 0.5;
    var total = 0.0;
    var at = x;
    for (var i = 0; i < 3; i = i + 1) {
        sum = sum + amplitude * mix(hash1(floor(at)), hash1(floor(at) + 1.0),
            smoothstep(0.0, 1.0, fract(at)));
        total = total + amplitude;
        at = at * 2.07;
        amplitude = amplitude * 0.5;
    }
    return sum / total;
}

/// Rise, hold, fall over a fraction of a period — the rail's own event shape.
///
/// The same two lines as `card_common.wgsl`'s `mark_event`, written again
/// rather than imported: that module is the cards' and pulling it in here
/// would drag a card's whole alphabet into the table's shader for a
/// four-token function.
fn event_envelope(f: f32, rise: f32, hold: f32, fall: f32) -> f32 {
    return smoothstep(0.0, rise, f) - smoothstep(hold, hold + fall, f);
}

/// A rounded rectangle's signed distance: negative inside, positive outside.
fn sd_round_box(p: vec2<f32>, half: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - half + vec2<f32>(r);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

fn inlay_colour(at: f32) -> vec3<f32> {
    let part = fract(at) * 5.0;
    if part < 1.0 { return INLAY_WHITE; }
    if part < 2.0 { return INLAY_BLUE; }
    if part < 3.0 { return INLAY_BLACK; }
    if part < 4.0 { return INLAY_RED; }
    return INLAY_GREEN;
}

fn hairline(distance: f32, width: f32, pixel: f32) -> f32 {
    return 1.0 - smoothstep(width, width + pixel, abs(distance));
}

/// The cloth at a point of table, in display-referred colour.
fn baize_at(p: vec2<f32>, footprint: vec2<f32>) -> vec3<f32> {
    // Big soft blotches of wear, then a fine grain and the weave on top —
    // the same three layers the CPU reference mixes, at table frequencies
    // rather than texel ones.
    let wear = fbm2(p * 0.085 + 3.1);
    let grain = fbm2(p * 4.2 + 17.9);
    let weave = sin(p.x * WEAVE * TAU) * sin(p.y * WEAVE * TAU) * 0.5 + 0.5;
    let resolved = vec2<f32>(1.0) - smoothstep(vec2<f32>(0.2), vec2<f32>(0.5), footprint * WEAVE);

    var colour = mix(FELT_CLOTH, FELT_WORN, pow(wear, 1.6));
    let lift = (grain - 0.5) * 0.022 + (weave - 0.5) * 0.006 * resolved.x * resolved.y;
    // A barely raised mineral vein breaks up the broad surface; it is fixed
    // in the cloth, unlike the light passing over the separate metal inlay.
    let vein = pow(1.0 - abs(sin(p.x * 0.37 + p.y * 0.61 + wear * 8.0)), 12.0);
    return colour + vec3<f32>(lift + vein * 0.009);
}

// ---------------------------------------------------------------------------
// The firewheel: five flames where the colour wheel used to be.
//
// `baylee_client_core::firewheel` is normative on every number below and
// carries the arguments; `table::shader_tests` reads these constants back out
// of this text and fails when the two drift. What is written here rather
// than there is only the drawing.
//
// The flames are painted **flat into the cloth**, which is the one decision
// worth repeating at the point of use. The camera never moves and looks down
// from about 20°, so a standing flame's height projects at `sin` of that and
// a flame lying in the table plane at `cos` of it — 2.7 times the picture for
// the same number, with no parallax lost, because a fixed eye never had any
// to lose. What makes it read as standing is the ground cue: the foot is the
// brightest and roundest part, the body rises from it along one shared
// screen-up direction, and the pool of light is centred on the foot and not
// on the flame.
// ---------------------------------------------------------------------------

const FIRES: u32 = 5u;
const FLAME_BLACK: u32 = 2u;
const FOOT_RADIUS: f32 = 0.68;
const FLAME_REACH: f32 = 1.6;
const HEIGHT_FLOOR: f32 = 0.40;
const HEIGHT_SPAN: f32 = 0.42;
const WIDTH_FLOOR: f32 = 0.10;
const WIDTH_SPAN: f32 = 0.05;
const EROSION_FLOOR: f32 = 0.35;
const DRAUGHT_PERIOD: f32 = 9.5;
const DRAUGHT_DEPTH: f32 = 0.05;
const FLARE_PERIOD: f32 = 7.3;
const PINCH_PERIOD: f32 = 8.3;
const GUTTER_PERIOD: f32 = 13.7;
const STILL_AT: f32 = 2.3;
const POOL_INNER: f32 = 0.20;
const POOL_OUTER: f32 = 0.80;
const POOL_INNER_GAIN: f32 = 0.100;
const POOL_OUTER_GAIN: f32 = 0.042;
const BLACK_BODY: vec3<f32> = vec3<f32>(0.32, 0.30, 0.38);
const BLACK_RIM: vec3<f32> = vec3<f32>(0.62, 0.52, 0.78);
const BLACK_RIM_FROM: f32 = 0.15;
const BLACK_RIM_TO: f32 = 0.90;
const BLACK_POOL: vec3<f32> = vec3<f32>(0.30, 0.24, 0.42);
const BLACK_POOL_GAIN: f32 = 0.35;
const BLACK_SHADOW: f32 = 0.62;
const BLACK_SHADOW_REACH: f32 = 0.30;
/// How many features of shape noise a flame carries per table unit.
const FLAME_GRAIN: f32 = 7.0;
const SOLE_INNER: f32 = 0.035;
const SOLE_OUTER: f32 = 0.075;
const SOLE_HEAT: f32 = 0.35;
const VEIL: f32 = 0.62;
const EROSION_GAIN: f32 = 2.0;

/// The hottest part of a flame, low in its body.
fn flame_core(i: u32) -> vec3<f32> {
    if i == 0u { return vec3<f32>(1.00, 0.98, 0.90); }
    if i == 1u { return vec3<f32>(0.86, 0.94, 1.00); }
    if i == 2u { return vec3<f32>(0.42, 0.30, 0.55); }
    if i == 3u { return vec3<f32>(1.00, 0.90, 0.70); }
    return vec3<f32>(0.90, 1.00, 0.82);
}

/// The colour a flame is read as. Black's is never painted — see `firewheel`.
fn flame_body(i: u32) -> vec3<f32> {
    if i == 0u { return vec3<f32>(0.94, 0.88, 0.70); }
    if i == 1u { return vec3<f32>(0.36, 0.62, 0.92); }
    if i == 2u { return vec3<f32>(0.00, 0.00, 0.00); }
    if i == 3u { return vec3<f32>(0.90, 0.38, 0.22); }
    return vec3<f32>(0.40, 0.72, 0.42);
}

/// The cooling skirt, which shifts hue and not only value.
fn flame_skirt(i: u32) -> vec3<f32> {
    if i == 0u { return vec3<f32>(0.85, 0.55, 0.22); }
    if i == 1u { return vec3<f32>(0.18, 0.24, 0.62); }
    if i == 2u { return BLACK_RIM; }
    if i == 3u { return vec3<f32>(0.55, 0.10, 0.20); }
    return vec3<f32>(0.10, 0.36, 0.34);
}

/// Each flame's own gutter rate in Hz, its depth, its scroll and how hard
/// the noise eats its tip: `(rate, depth, scroll, erosion)`.
///
/// The rates are written to three places because round tenths are not
/// mutually prime: `[0.9, 1.4, 0.6, 1.2, 0.8]` brings the whole wheel back
/// to where it started every ten seconds, for as long as the game lasts.
fn flame_beat(i: u32) -> vec4<f32> {
    if i == 0u { return vec4<f32>(0.907, 0.05, 1.6, 2.2); }
    if i == 1u { return vec4<f32>(1.433, 0.03, 2.2, 2.2); }
    if i == 2u { return vec4<f32>(0.581, 0.10, 1.6, 2.8); }
    if i == 3u { return vec4<f32>(1.193, 0.09, 1.6, 2.2); }
    return vec4<f32>(0.769, 0.06, 1.6, 2.2);
}

/// Where flame `i`'s foot stands, and how hard it burns: `(x, y, strength)`.
///
/// White at the top of the screen, then clockwise — the wheel as a card
/// prints it.
fn flame_at(i: u32) -> vec3<f32> {
    let angle = 1.5707964 - 6.2831855 * f32(i) / f32(FIRES);
    var s = params.flames_tail.x;
    if i == 0u { s = params.flames.x; }
    else if i == 1u { s = params.flames.y; }
    else if i == 2u { s = params.flames.z; }
    else if i == 3u { s = params.flames.w; }
    return vec3<f32>(cos(angle) * FOOT_RADIUS, sin(angle) * FOOT_RADIUS, s);
}

/// The wheel, painted into already-lit cloth.
///
/// The order is the whole of it and it goes wrong silently: black's body is
/// a **multiply** and has to land before anything is added; the four painted
/// bodies **replace**, after the sky, or a white flame turns blue at night;
/// and the pools are **added last, in linear**, the way the rail lamp's glow
/// is, so the weave and the mineral vein show through the light instead of
/// being covered by it.
fn firewheel(start: vec3<f32>, table: vec2<f32>, pixel: f32, t: f32) -> vec3<f32> {
    let up = normalize(params.flames_tail.yz);
    let right = vec2<f32>(up.y, -up.x);
    var lit = start;
    var pool = vec3<f32>(0.0);

    // The room's own air, shared by all five. It is what makes them five
    // flames in one room rather than five looping pictures side by side.
    let draught = 1.0 + DRAUGHT_DEPTH * sin(t * TAU / DRAUGHT_PERIOD);

    for (var i = 0u; i < FIRES; i = i + 1u) {
        let fire = flame_at(i);
        let s = clamp(fire.z, 0.0, 1.0);
        let p = table - fire.xy;
        let d = length(p);
        let beat = flame_beat(i);
        let seed = f32(i) * 17.3;

        // A fed fire puffs *slower* and deeper, never faster: faster is
        // exactly the peripheral motion the rest of this table refuses.
        let rate = beat.x * inverseSqrt(0.5 + s);
        let depth = beat.y * (0.6 + 0.4 * s);
        var envelope = draught * (1.0 + depth * (fbm1(t * rate + seed) - 0.5) * 2.0);

        // One event each, on its own period, for the three colours whose
        // character is an event rather than a rate.
        var heat_gift = 0.0;
        var erode_gift = 0.0;
        if i == 3u {
            // Red flares, and a fed fire flares more often.
            let flare = event_envelope(fract(t * (0.6 + 0.4 * s) / FLARE_PERIOD), 0.03, 0.05, 0.30);
            envelope = envelope * (1.0 + 0.22 * flare);
            heat_gift = 0.30 * flare;
        } else if i == 0u {
            // White gutters down and recovers, and a fed candle gutters less.
            let gutter = event_envelope(fract(t / (GUTTER_PERIOD * (0.6 + 0.4 * s))), 0.05, 0.08, 0.25);
            envelope = envelope * (1.0 - 0.20 * gutter);
        } else if i == FLAME_BLACK {
            // Black sheds its tip as smoke.
            erode_gift = 0.25 * event_envelope(fract(t / PINCH_PERIOD), 0.10, 0.15, 0.30);
        }

        // The light it throws, centred on the **foot** — a pool centred on
        // the body is a decal of the flame — and rising and falling with the
        // flame itself, with no lag, which is the single strongest cue that
        // the light comes from the fire.
        let near = 1.0 - smoothstep(0.0, POOL_INNER, d);
        let far = 1.0 - smoothstep(0.0, POOL_OUTER, d);
        let lightness = (0.5 + 0.5 * s) * envelope;
        let amount = POOL_INNER_GAIN * near * near + POOL_OUTER_GAIN * far * far;
        if i == FLAME_BLACK {
            // The additive slot cannot darken, so black's light is two
            // stages: a shadow at the foot, then a cold pool over it.
            let shade = 1.0 - smoothstep(0.0, BLACK_SHADOW_REACH, d);
            lit = lit * mix(vec3<f32>(1.0), vec3<f32>(BLACK_SHADOW), shade * shade);
            pool = pool + to_linear(BLACK_POOL) * BLACK_POOL_GAIN * lightness * amount;
        } else {
            pool = pool + to_linear(mix(flame_body(i), flame_core(i), 0.4)) * lightness * amount;
        }

        // Nothing of the body reaches this far out; the pools above already
        // did their work, so the rest of the flame can be skipped.
        if d > POOL_INNER + HEIGHT_FLOOR + HEIGHT_SPAN {
            continue;
        }

        let h = max((HEIGHT_FLOOR + HEIGHT_SPAN * s) * envelope, 0.05);
        let w = WIDTH_FLOOR + WIDTH_SPAN * s;
        let x = dot(p, right);
        let y = dot(p, up);
        let v = y / h;
        let climb = clamp(v, 0.0, 1.0);

        // The shape noise scrolls *up* the flame, which is where the sense
        // of burning lives — the silhouette moves, the inside stays smooth.
        let step_shape = pixel * FLAME_GRAIN;
        let n = fbm2_faded(vec2<f32>(x, y) * FLAME_GRAIN - vec2<f32>(0.0, t * beat.z) + seed, step_shape);
        var warp = (fbm2_faded(vec2<f32>(x, y) * 6.0 - vec2<f32>(0.0, t * 1.1 * beat.z) + seed + 31.0, pixel * 6.0) - 0.5)
            * 0.16 * pow(climb, 1.5);
        if i == 4u {
            // Green wanders instead of flaring: the tip leans, slowly.
            warp = warp + 0.06 * (fbm1(t * 0.35 + seed) - 0.5) * 2.0 * climb;
        }

        // The silhouette: a candle's, not a teardrop's. It opens fast off
        // the foot and then tapers the whole rest of the way, which is the
        // difference between a flame and a balloon — `4v(1-v)` is symmetric
        // about the middle and drew five gel capsules.
        let profile = w * sqrt(clamp(v * 5.0, 0.0, 1.0)) * pow(max(1.0 - climb, 0.0), 0.75);
        let live = v > 0.0 && v < 1.0;
        let inside = select(-1.0, 1.0 - abs(x + warp) / max(profile, 1e-4), live);
        // ...eaten from the top down. Where a high band of noise crosses with
        // a low one above it the shape pinches and a tongue comes off, and at
        // thirty pixels that lick is the one thing that reads as fire.
        //
        // `climb` and not `v`: above and below the flame `v` keeps growing,
        // and squared against a noise value under the floor the term changes
        // sign and paints a positive field — the first build scattered
        // coloured confetti over the whole middle of the table, and every
        // one of those specks was this line.
        let eroded = inside - (beat.w + erode_gift) * climb * climb
            * (n - EROSION_FLOOR) * EROSION_GAIN;
        // The field runs 1 to 0 across a half-width, so a pixel is this much
        // of it. Unclamped it goes to infinity at the foot and at the tip,
        // where the profile is zero.
        let aa = clamp(pixel / max(profile, 1e-4), 0.02, 0.5);
        let sole = 1.0 - smoothstep(SOLE_INNER, SOLE_OUTER, d);
        let body = select(0.0, smoothstep(-aa, aa, eroded), live);
        let mask = max(body, sole);
        if mask <= 0.0 {
            continue;
        }

        // Hot low and at the axis, cool high and at the edge — and at zero
        // strength there is no core at all, which is what a pilot light is.
        let heat = pow(clamp(inside, 0.0, 1.0), 0.8) * (1.0 - 0.75 * climb) * (0.8 + 0.5 * s)
            + SOLE_HEAT * sole + heat_gift;
        // Opaque where it is hot and thin where it is cooling, which is what
        // you can see the wall through a real flame at.
        let veil = mask * mix(VEIL, 1.0, smoothstep(0.0, 0.5, heat));

        if i == FLAME_BLACK {
            // A hole in the cloth, with the light on its edge. The body is
            // never painted: on a dark table nothing can emit black, so what
            // is drawn is the felt darkened, and what says the shape is the
            // rim — which is how a backlit black flame is drawn everywhere
            // it is drawn well.
            lit = lit * mix(vec3<f32>(1.0), BLACK_BODY, veil);
            lit = mix(lit, to_linear(flame_core(i)), mask * smoothstep(0.55, 0.95, heat) * 0.7);
            // Gated on `live` for the same reason the body is: `|eroded|`
            // passes through zero all over the cloth, and an ungated rim
            // drew violet lace across the middle of the table.
            let rim = select(
                0.0,
                (1.0 - smoothstep(0.0, 1.5 * aa, abs(eroded)))
                    * smoothstep(BLACK_RIM_FROM, BLACK_RIM_TO, climb),
                live,
            );
            lit = mix(lit, to_linear(BLACK_RIM), rim);
        } else {
            var col = mix(flame_skirt(i), flame_body(i), smoothstep(0.0, 0.55, heat));
            col = mix(col, flame_core(i), smoothstep(0.55, 0.95, heat));
            lit = mix(lit, to_linear(col), veil);
        }
    }

    return lit + pool;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Table space out of the world position, rather than the mesh's own uv.
    // The uv origin is a convention of whichever builder made the mesh, and a
    // wrong guess about it mirrors the whole field — invisible at two seats,
    // because a duel is symmetric about both axes, and wrong at three.
    // `to_world` is `(x, height, -y)`, so this is exactly its inverse.
    let table = vec2<f32>(in.world_position.x, -in.world_position.z);
    // Derivatives precede the surface branches, including the apron return.
    let footprint = fwidth(table);
    let pixel = max(length(footprint), 0.001);
    let half = params.span * 0.5;

    // The apron: the wall of the slab. Told apart by its normal, which is the
    // only face that does not point up.
    if (in.world_normal.y < 0.5) {
        // Down the height, darker as it goes: the underside of a table is in
        // its own shadow, and nothing here lights it.
        let drop = clamp(-in.world_position.y / max(params.thickness, 1e-3), 0.0, 1.0);
        // And a little brighter on the near side, which is the one edge of a
        // table anybody ever sees at this camera.
        let faces = clamp(in.world_normal.z * 0.5 + 0.5, 0.0, 1.0);
        let grain = fbm2(vec2<f32>(table.x + table.y, in.world_position.y * 6.0) * 2.0);
        let shade = mix(1.15, 0.42, drop) * mix(0.78, 1.0, faces);
        let trim = 1.0 - smoothstep(0.025, 0.09, abs(drop - 0.28));
        let apron = to_linear(APRON * shade + vec3<f32>((grain - 0.5) * 0.012) + ENGRAVING * trim * 0.22);
        return vec4<f32>(under_weather(under_sky(under_lamp(apron, table))), 1.0);
    }

    // One field decides the whole top. `outer` is the mesh's own boundary
    // (zero at its edge); the felt is that shape inset by one rail.
    let outer = sd_round_box(table, half, params.corner);
    let edge = outer + params.rail;
    // How far inside the cloth a point is, in table units. Positive on felt.
    let inset = -edge;

    var colour: vec3<f32>;
    var lamp_here = 0.0;

    if (inset > 0.0) {
        // The cloth, and the crest of the roll it runs over at the edge:
        // brighter towards the boundary, not darker. `FELT_DEEP` is still the
        // shadow the middle of a big table falls into — that is the lamp
        // below — and no longer a ring painted round the play area.
        let crest = 1.0 - smoothstep(0.0, ROLL, inset);
        colour = baize_at(table, footprint) + vec3<f32>(crest * ROLL_LIGHT);
        // A hair of the lamp spills off the rail onto the cloth beside it,
        // and no further. Without it the rail reads as a sticker.
        lamp_here = crest * 0.35;
    } else {
        // The roll: it leaves the cloth at the cloth's own height and turns
        // down and outward to meet the apron, so the light on it sits at the
        // **inner** edge and everything after that is falling away. A rail
        // crowned in its own middle draws a bead all the way round, which is
        // a picture frame with the felt inside it; a roll that only ever
        // falls is an edge the felt runs over.
        let across = clamp(-inset / max(params.rail, 1e-3), 0.0, 1.0);
        // A quarter circle's cosine rather than a straight ramp: the surface
        // is turning away from the eye, and a linear fall reads as a chamfer
        // cut at forty-five degrees.
        let turn = sqrt(max(1.0 - across * across, 0.0));
        let hide = vnoise(table * vec2<f32>(1.8, 72.0) + 41.3);
        colour = mix(RAIL_HIDE, RAIL_LIP, turn * 0.75) + vec3<f32>((hide - 0.5) * 0.018);
        let bevel = hairline(-outer - 0.09, 0.018, pixel);
        let inner_bevel = hairline(inset + 0.07, 0.012, pixel);
        colour += ENGRAVING * (bevel * 0.45 + inner_bevel * 0.25);
        lamp_here = 0.20 + 0.45 * turn;
    }

    // Twin engraved circuits frame the playfield, outside the cards. Their
    // colour stays in place while a slow change in luminosity reveals depth.
    let circuit = hairline(inset - 0.19, 0.012, pixel);
    let outer_circuit = hairline(inset - 0.32, 0.008, pixel);
    let around = atan2(table.y / half.y, table.x / half.x) / TAU + 0.5;
    let jewel = inlay_colour(around);
    let glint = 0.70 + 0.30 * sin(around * TAU * 2.0 - globals.time * params.motion * 0.22);
    let section = fract(around * 60.0);
    let etch = smoothstep(0.10, 0.20, section) * (1.0 - smoothstep(0.65, 0.75, section));
    colour = mix(colour, ENGRAVING, outer_circuit * 0.28);
    colour = mix(colour, jewel, circuit * glint * 0.55);
    colour += ENGRAVING * etch * hairline(inset - 0.255, 0.035, pixel) * 0.13;

    // An engraved compass around the firewheel. Low contrast and static; no
    // animated wash ever crosses card artwork.
    let radius = length(table);
    let compass = hairline(radius - 1.30, 0.008, pixel);
    let breaks = pow(abs(sin(atan2(table.y, table.x) * 10.0)), 14.0);
    let ticks = hairline(radius - 1.41, 0.065, pixel) * breaks;
    colour = mix(colour, ENGRAVING, (compass * 0.22 + ticks * 0.16) * smoothstep(0.4, 0.8, inset));

    // The wheel's own two rings, in worn gold. They were a 512-texel quad
    // inlaid over the felt and are hairlines in the cloth now: sharper (that
    // texture was 2.4 texels to a physical pixel at this camera) and, more
    // to the point, **underneath**. A ring blended over a flame etches it;
    // three of the five cross the outer ring, and a flame standing in front
    // of the rim is the picture.
    //
    // The inner ring came in from 0.33 to 0.26 when the flames arrived: the
    // black and red flames rise toward the middle and their inner flank
    // passes about 0.30, so at 0.33 two of them would have cut it.
    let tarnish = 0.75 + 0.45 * fbm2(table * 6.0);
    let rings = clamp(
        hairline(radius - 1.03, 0.017, pixel) * 0.85
            + hairline(radius - 0.26, 0.010, pixel) * 0.55,
        0.0,
        1.0,
    );
    colour = mix(colour, GILT * tarnish, rings);

    // The lamp. It enters at the active seat's own edge and runs round the
    // rail, so combat begins on the attacker's side of the table and reaches
    // the defender — the seam between the two is the moment itself, not an
    // ornament that is always there.
    let along = dot(table - params.source.xy, params.source.zw) / max(params.span.y, 1.0);
    let near_bank = 1.0 - smoothstep(-0.15, 0.85, along);
    let reach = mix(0.40, 1.0, near_bank);

    // Energy to the fourth, and that is what lets combat bloom while a main
    // phase stays a quiet amber line. `phase_light` grades its lamps the way
    // a colour is graded — by eye, on a screen — so the step from a main
    // phase (0.16) to combat damage (1.0) is a factor of six in a
    // display-referred number and nearer forty in light. Squaring alone was
    // tried and does the loud end but not the quiet one: untap grades at 0.45,
    // which squares to 0.20, and at 0.20 the table was the brightest thing on
    // itself in the step where nothing whatever happens.
    let e2 = params.wash.a * params.wash.a;
    let energy = max(e2 * e2, RESTING);
    let tint = smoothstep(0.02, 0.30, energy);
    let lamp = to_linear(mix(RAIL_LIGHT, params.wash.rgb, tint));

    // A slow pulse travelling round the rail, so the table is alive without
    // ever being a thing that moves while a card is being read.
    let turn = atan2(table.y, table.x) / TAU;
    let pulse = 0.5 + 0.5 * sin(
        turn * TAU * PULSE_TURNS - globals.time * params.motion * PULSE_SPEED
    );

    let glow = lamp * energy * params.gain * reach * lamp_here * (0.62 + 0.38 * pulse);
    // The lamp reaches the table's own colour and stops there: the phase
    // light is something the table *emits*, and a step that dimmed towards
    // the ends of the slab would be saying something untrue about the turn.
    var lit = under_weather(under_sky(under_lamp(to_linear(colour), table)));

    // The firewheel, inside its own reach and nowhere else. The branch is
    // the budget as well as the shape: five flames cost nothing at all over
    // the other 99% of a thirty-five-unit slab.
    //
    // It is applied to *lit* cloth rather than to `colour`, which is not a
    // detail — a flame is something the table emits, so a white one would go
    // blue at night if it were graded by the sky like the cloth under it.
    if radius < FLAME_REACH {
        lit = firewheel(lit, table, pixel, mix(STILL_AT, globals.time, params.motion));
    }
    return vec4<f32>(lit + glow, 1.0);
}
