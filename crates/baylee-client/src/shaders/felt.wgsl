// The table the game is played on: midnight mineral cloth inside a machined
// bronze rail, with engraved inlays and a shadowed apron below it.
//
// No texture at all. The slab is about thirty-five units across and a card is
// roughly 114 physical pixels at this camera, so drawing the cloth sharply
// would want some four thousand texels — and measured in a debug build,
// generating 2048 already costs 1.6 seconds every time the table is re-cut.
// Arithmetic has no resolution. The one texture it reads is not a picture
// but a table of numbers: every vein cell's point, which the arithmetic
// would otherwise work out again nine times per field on every pixel
// (`vein_points`, `baylee_client_core::feltveins`).
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
#import "embedded://baylee_client/shaders/noise.wgsl"::{hash_cell}

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
    /// How many seats stand on the dial (0 to 8).
    seats: f32,
    pattern: vec4<f32>,
    /// The texel offset of each cellular field's cells in `vein_points`:
    /// trunk in `xy`, capillary in `zw`.
    veins: vec4<f32>,
    /// The two hands, as table-space vectors (`baylee_client_core::dial`):
    /// the turn hand in `xy`, the priority hand in `zw`.
    hands: vec4<f32>,
    /// `x` the priority hand's length (0 retracted, 1 whole), `y` whether it
    /// points at me, `z` when the turn hand arrived, `w` when the priority
    /// hand did — on `globals.time`'s clock.
    dial: vec4<f32>,
    /// `x` when the hub last pulsed, `y` which light (1 ivory, 2 teal, 0
    /// none), `z` the dial's scale (`baylee_client_core::dial::scale_for`;
    /// 0 reads as 1).
    pulse: vec4<f32>,
    /// Each seat's jewel direction, two per vector: `xy` then `zw`.
    jewels: array<vec4<f32>, 4>,
    /// Each seat's jewel colour (display-referred) in `rgb`; `a` 1 for a seat
    /// at the table, 0.4 for one that has left, plus 2 while it is choosing
    /// its opening hand (the mulligan arc).
    tints: array<vec4<f32>, 8>,
    /// A team's colour round the jewel, `a` 1 where the seat has a team.
    teams: array<vec4<f32>, 8>,
    /// The tear (DESIGN-v8, the owner's of 07.10.2026; `table::pieces`):
    /// `w` what this is — 0 the whole slab, 1 a piece of the tearing table,
    /// 2 the void under it, 3 the dial lifted off it; `x` how far the veins
    /// spill out of the cut (0 to 1), `y` the jagged line's seed, `z` the
    /// weld seam's heat (0 to 1). The whole slab holds zero: `w` is the gate
    /// every bit of this work stands behind.
    rift: vec4<f32>,
    /// Where the baked slow fields lie (`baylee_client_core::feltwarp`):
    /// `xy` the table point at the grid's top-left corner, `zw` one over its
    /// size. All zero until the grid has been computed for this cut, and
    /// then the fields are worked out here instead.
    warp: vec4<f32>,
    /// Where each hand pointed a moment ago in its sweep (`Hand::trail`):
    /// the turn hand's in `xy`, the priority hand's in `zw`; the hand's own
    /// direction (or zero) when it is still, and then no trail is drawn.
    trail: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> params: FeltParams;

// The molten seam's colours, display-referred: emitted, never lit — white
// hot at contact, through orange, to a dark red that goes out.
const SEAM_HOT: vec3<f32> = vec3<f32>(1.0, 0.78, 0.42);
const SEAM_ORANGE: vec3<f32> = vec3<f32>(1.0, 0.42, 0.08);
const SEAM_DARK: vec3<f32> = vec3<f32>(0.28, 0.035, 0.01);
// The cut's cross-section: the glass layer on top, the body under it, and
// the veins cut through it as coloured streaks.
const CUT_GLASS: vec3<f32> = vec3<f32>(0.16, 0.22, 0.25);
const CUT_BODY: vec3<f32> = vec3<f32>(0.026, 0.028, 0.032);
const CUT_RIM: vec3<f32> = vec3<f32>(0.62, 0.70, 0.72);
const STREAK_LAVA: vec3<f32> = vec3<f32>(0.55, 0.15, 0.03);
const STREAK_WATER: vec3<f32> = vec3<f32>(0.08, 0.27, 0.34);
// The two rivers as they run out of the cut: lava from white-hot to a dark
// glow, water translucent with a bright rim.
const LAVA_HOT: vec3<f32> = vec3<f32>(1.0, 0.66, 0.20);
const LAVA_COOL: vec3<f32> = vec3<f32>(0.42, 0.05, 0.02);
const WATER: vec3<f32> = vec3<f32>(0.16, 0.48, 0.60);
const WATER_RIM: vec3<f32> = vec3<f32>(0.70, 0.90, 0.95);
// The void under a tearing table.
const VOID_DEEP: vec3<f32> = vec3<f32>(0.006, 0.008, 0.012);
const VOID_MIST: vec3<f32> = vec3<f32>(0.030, 0.040, 0.052);
// The falling ribbons: one to a cell this wide along the cut, this wide at
// the top edge (each side of its middle), table units.
const RIBBON_CELL: f32 = 0.7;
const RIBBON_WIDTH: f32 = 0.15;
// How far below the slab the drips may hang, in slab thicknesses
// (`table::pieces::DRIP_DEPTH`).
const DRIP_DEPTH: f32 = 1.6;
// The floating dial's radius (`table::pieces::DIAL_R`): a piece of the
// tearing table draws the dial's empty bed inside it.
const DIAL_R: f32 = 1.6;

/// A ribbon of a cut river falling from the cut's top edge at `x`, `down`
/// slab thicknesses below it: `x` its strength (0 outside it, 1 at its
/// core), `y` how near its edge it is (1 at the rim, for water's bright
/// edge), `z` the share of lava in it, `w` how far along its fall.
///
/// The ribbons stand along the cut one to a cell of `RIBBON_CELL`, each at
/// its own place in its cell, and a ribbon runs only where the river it
/// would carry is cut at its place: so it is one colour from top to tip,
/// and the streams are as many as the veins the tear crosses, no more. A
/// ribbon falls as long as the spill has run (`rift.x`), thins as a falling
/// stream does — fast under the edge, slowly after, as it speeds up — and
/// breaks into drops at its tip.
fn ribbon(x: f32, down: f32, t: f32) -> vec4<f32> {
    let total = 1.0 + DRIP_DEPTH;
    if (params.rift.x <= 0.0) {
        return vec4<f32>(0.0);
    }
    let cell = floor(x / RIBBON_CELL);
    let key = vec2<i32>(i32(cell), i32(params.rift.y * 7.0));
    let place = hash_cell(key);
    let centre = (cell + 0.25 + 0.5 * place) * RIBBON_CELL;
    let width = RIBBON_WIDTH / sqrt(1.0 + 1.5 * max(down, 0.0));
    let off = abs(x - centre);
    if (off > width) {
        return vec4<f32>(0.0);
    }
    // Each ribbon reaches its own length, all of it only at full spill.
    let reach = params.rift.x * total * (0.5 + 0.5 * hash_cell(key + vec2<i32>(0, 1)));
    if (down > reach) {
        return vec4<f32>(0.0);
    }
    let edge = veins_at(vec2<f32>(centre, tear_line(centre, params.rift.y)));
    if (max(edge.water, edge.lava) < 0.55) {
        return vec4<f32>(0.0);
    }
    let lava_share = clamp(edge.lava / max(edge.lava + edge.water, 1e-3), 0.0, 1.0);
    let core = 1.0 - smoothstep(width * 0.55, width, off);
    let flow = sqrt(max(down, 0.0)) * 3.2 - t * 1.7;
    // Drops: near the tip the ribbon breaks into beads that fall with it.
    let tip = smoothstep(reach * 0.6, reach, down);
    let beads = smoothstep(0.35, 0.5, fract(flow * 1.4 + place * 3.0));
    let strength = core * mix(1.0, beads, tip);
    let rim = smoothstep(width * 0.35, width * 0.8, off);
    return vec4<f32>(strength, rim, lava_share, down / max(reach, 1e-3));
}

/// The colour of a falling ribbon, in linear light.
fn liquid(r: vec4<f32>, x: f32, down: f32, t: f32) -> vec3<f32> {
    let flow = sqrt(max(down, 0.0)) * 3.2 - t * 1.7;
    let pulse = 0.65 + 0.35 * vnoise(vec2<f32>(x * 7.0, flow * 4.0));
    // Lava cools as it falls: white-hot at the cut, a dark glow at the tip.
    let lava = to_linear(mix(LAVA_HOT, LAVA_COOL, smoothstep(0.0, 1.0, r.w))) * (0.75 + 0.5 * pulse);
    // Water is seen through: dim in its body, bright at its rim.
    let water = to_linear(WATER) * (0.45 + 0.3 * pulse) + to_linear(WATER_RIM) * r.y * 0.55;
    return mix(water, lava, step(0.5, r.z));
}

/// A piece's cut face (`table::pieces::geometry`): `u` (2 to 3) across the
/// table, `v` 0 at the top, 1 at the slab's bottom, past 1 the drips. A
/// solid wall: the glass layer on top with a bright edge, the dark body
/// under it with the veins cut through it as coloured streaks, and over it
/// and on below it, while the table is open, the cut rivers falling.
fn cut_face(uv: vec2<f32>) -> vec4<f32> {
    let t = globals.time * params.motion;
    let x = (uv.x - 2.5) * params.span.x;
    let line = tear_line(x, params.rift.y);
    let v = uv.y;
    let fall = ribbon(x, v, t);
    if (v <= 1.0) {
        // The veins run on into the body below the cut: the field read on
        // into the table across the line, as deep as the face goes.
        let deep = veins_at(vec2<f32>(x, line + v * 1.6));
        let glass = 1.0 - smoothstep(0.13, 0.17, v);
        var lit = under_sky(to_linear(mix(CUT_BODY, CUT_GLASS, glass)));
        // The glass is a little translucent: the veins under it show
        // through, dimmed.
        lit += to_linear(STREAK_LAVA) * deep.lava * glass * 0.35;
        lit += to_linear(STREAK_WATER) * deep.water * glass * 0.35;
        // Its polished top edge catches the light.
        lit += to_linear(CUT_RIM) * (1.0 - smoothstep(0.0, 0.035, v)) * 0.55;
        let body = 1.0 - glass;
        lit = mix(lit, to_linear(STREAK_LAVA), deep.lava * body * 0.85);
        lit = mix(lit, to_linear(STREAK_WATER), deep.water * body * 0.75);
        if (fall.x > 0.0) {
            let liquid_at = liquid(fall, x, v, t);
            // Water lets the wall show through; lava does not.
            let cover = mix(0.6, 0.95, step(0.5, fall.z));
            lit = mix(lit, liquid_at, clamp(fall.x, 0.0, 1.0) * cover);
        }
        return vec4<f32>(lit, 1.0);
    }
    if (fall.x < 0.5) {
        discard;
    }
    return vec4<f32>(liquid(fall, x, v, t), 1.0);
}

/// A piece's share of the rim's wall (`table::pieces::geometry`): `u` (4 to
/// 5) across the table, `v` how far down. Shaded as the slab's apron is,
/// from the mesh rather than from the world: a piece lifted or sunk is the
/// same wall.
fn piece_wall(uv: vec2<f32>, normal: vec3<f32>) -> vec4<f32> {
    let x = (uv.x - 4.5) * params.span.x;
    // Table space's `y` of the wall: its side of the table, from its normal
    // (`to_world` is `(x, height, -y)`).
    let table = vec2<f32>(x, clamp(-normal.z, -1.0, 1.0) * params.span.y * 0.5);
    let faces = clamp(normal.z * 0.5 + 0.5, 0.0, 1.0);
    return vec4<f32>(apron_lit(table, clamp(uv.y, 0.0, 1.0), faces), 1.0);
}

/// The weld seam on my piece (`table::pieces::geometry`): `u` (6 to 7)
/// along the line, `v` across it. A thin line on the jag, emitted: white-hot
/// as the pieces meet, cooling through orange and dark red to nothing
/// (`rift.z`), and nothing at all while it is cold.
fn seam_strip(uv: vec2<f32>) -> vec4<f32> {
    let heat = clamp(params.rift.z, 0.0, 1.0);
    let across = abs(uv.y * 2.0 - 1.0);
    let core = 1.0 - smoothstep(0.15, 1.0, across);
    if (heat * core < 0.03) {
        discard;
    }
    var colour = mix(SEAM_DARK, SEAM_ORANGE, smoothstep(0.0, 0.55, heat));
    colour = mix(colour, SEAM_HOT, smoothstep(0.7, 1.0, heat) * core);
    return vec4<f32>(to_linear(colour) * (0.5 + heat), 1.0);
}

/// The void under a tearing table (`table::pieces::geometry::void_mesh`):
/// dark, a slow mist drifting in it, and under the open cut the glow of the
/// lava spilling into it (`rift.x`). Not lit by the room: it is where no
/// light reaches.
fn void_floor(uv: vec2<f32>) -> vec4<f32> {
    let t = globals.time * params.motion;
    let table = vec2<f32>((uv.x - 0.5) * params.span.x, (0.5 - uv.y) * params.span.y);
    let mist = fbm(table * 0.3 + vec2<f32>(t * 0.04, -t * 0.03));
    var lit = to_linear(mix(VOID_DEEP, VOID_MIST, smoothstep(0.35, 0.75, mist)));
    let off = abs(table.y - tear_line(table.x, params.rift.y));
    let embers = 0.6 + 0.4 * vnoise(table * 1.4 + vec2<f32>(0.0, t * 0.5));
    lit += to_linear(SEAM_ORANGE) * exp(-off * 0.8) * params.rift.x * embers * 0.06;
    return vec4<f32>(lit, 1.0);
}

/// Every vein cell's point, as `vein_distance` used to compute it with two
/// value noises per cell (`baylee_client_core::feltveins`, which computes it
/// once per cut with this file's own `vnoise`). Read with `textureLoad`: one
/// texel is one cell, never filtered.
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var vein_points: texture_2d<f32>;

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
/// The turn hand: ivory, v6's *am Zug*.
const IVORY: vec3<f32> = vec3<f32>(0.95, 0.91, 0.80);
/// The priority hand: teal, v6's *wartet* (`palette::ACCENT`).
const TEAL: vec3<f32> = vec3<f32>(0.33, 0.75, 0.71);
/// The dial (`baylee_client_core::dial`): compass, hub plate, stones' band.
const COMPASS_R: f32 = 1.30;
const HUB_R: f32 = 0.64;
const STONE_R: f32 = 0.875;
const STONE_SCALE: f32 = 0.80;
const TURN_TIP: f32 = 1.22;
const PRIO_TIP: f32 = 1.08;
const OUTLINE: f32 = 0.018;
/// The face's rim and the chapter band inside it, the jewels' radius, the
/// turn hand's counterweight and the priority spark's orbit (dial-v2;
/// `baylee_client_core::dial::DIAL_OUTER` is the rim).
const BEZEL_R: f32 = 1.40;
const BAND_IN: f32 = 1.19;
const JEWEL_R: f32 = 0.066;
const TAIL_END: f32 = 0.80;
const ORBIT: f32 = 0.12;

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

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let c0 = vec2<i32>(i);
    let a = hash_cell(c0);
    let b = hash_cell(c0 + vec2<i32>(1, 0));
    let c = hash_cell(c0 + vec2<i32>(0, 1));
    let d = hash_cell(c0 + vec2<i32>(1, 1));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

/// Four octaves, normalised back to 0..1 so callers can centre it on a half.
fn fbm(p: vec2<f32>) -> f32 {
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
// Polished smoked glass over slow elemental strata. Bounded noise work,
// no refraction buffer and no additional full-screen pass.
// Distance to a cellular seam. Two scales form connected trunks and finer
// capillaries; domain warping removes the straight polygon edges. Fixed loops
// and scalar noise keep the network in the existing opaque material pass.
fn vein_distance(p: vec2<f32>, texel_offset: vec2<f32>) -> f32 {
    let cell = floor(p);
    let local = fract(p);
    var nearest = 8.0;
    var second = 8.0;
    let base = vec2<i32>(cell + texel_offset);
    let last = vec2<i32>(textureDimensions(vein_points)) - vec2<i32>(1);
    for (var y = -1; y <= 1; y += 1) {
        for (var x = -1; x <= 1; x += 1) {
            let offset = vec2<f32>(f32(x), f32(y));
            let texel = clamp(base + vec2<i32>(x, y), vec2<i32>(0), last);
            let point = textureLoad(vein_points, texel, 0).xy;
            let delta = offset + point - local;
            let d = dot(delta, delta);
            second = min(second, max(nearest, d));
            nearest = min(nearest, d);
        }
    }
    return (sqrt(second) - sqrt(nearest)) * 0.5;
}

/// Where the water and the molten veins run at a point of the glass: the
/// warped domain, each river's distance and how much of it is there.
struct Veins {
    warp: vec2<f32>,
    water_d: f32,
    lava_d: f32,
    water: f32,
    lava: f32,
    silt: f32,
}

/// The slow fields, baked once per cut on the CPU with this file's own
/// arithmetic (`baylee_client_core::feltwarp`): the warp's two noises in
/// `xy`, the heat in `z`, the silt in `w`.
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var warp_field: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var warp_sampler: sampler;

/// The baked fields at table point `p`, or a negative `x` where there are
/// none (before the grid arrives, or past its edge).
fn baked_at(p: vec2<f32>) -> vec4<f32> {
    if (params.warp.z <= 0.0) {
        return vec4<f32>(-1.0);
    }
    let uv = vec2<f32>(p.x - params.warp.x, params.warp.y - p.y) * params.warp.zw;
    if (any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0))) {
        return vec4<f32>(-1.0);
    }
    // An explicit level: this is read inside branches, where implicit
    // derivatives are undefined.
    return textureSampleLevel(warp_field, warp_sampler, uv, 0.0);
}

fn veins_at(p: vec2<f32>) -> Veins {
    let angle = params.pattern.z * (6.2831853 / 256.0);
    let axis = vec2<f32>(cos(angle), sin(angle));
    let domain = vec2<f32>(dot(p, axis), dot(p, vec2<f32>(-axis.y, axis.x)))
        * (0.85 + params.pattern.w * (0.30 / 256.0)) + params.pattern.xy;
    // The warp, the heat and the silt are smooth and still: read them off
    // the baked grid where there is one, and work them out where not.
    let baked = baked_at(p);
    var warp: vec2<f32>;
    var heat: f32;
    var silt: f32;
    if (baked.x >= 0.0) {
        warp = domain + baked.xy * 3.4;
        heat = baked.z;
        silt = baked.w;
    } else {
        warp = domain + vec2<f32>(fbm(domain * 0.32), fbm(domain * 0.32 + 19.4)) * 3.4;
        heat = vnoise(warp * 0.19 + 42.0);
        silt = fbm(p * 0.34);
    }
    let trunk = vein_distance(warp * 0.28, params.veins.xy);
    let capillary = vein_distance(warp * 0.73 + 8.3, params.veins.zw);
    // Fine branches fade between the larger vessels instead of filling every
    // cell with equally bright cracks. Water and molten rock share junctions.
    let branch = min(trunk, capillary * 3.2 + 0.012 + smoothstep(0.04, 0.20, trunk) * 0.065);
    let water_d = branch * 18.0 + smoothstep(0.44, 0.64, heat) * 1.05;
    let lava_d = branch * 24.0 + (1.0 - smoothstep(0.36, 0.56, heat)) * 0.95;
    let water = 1.0 - smoothstep(0.42, 1.25, water_d);
    let lava = 1.0 - smoothstep(0.30, 0.98, lava_d);
    return Veins(warp, water_d, lava_d, water, lava, silt);
}

fn glass_at(p: vec2<f32>) -> vec3<f32> {
    let t = globals.time * params.motion;
    let field = veins_at(p);
    let warp = field.warp;
    let water_d = field.water_d;
    let lava_d = field.lava_d;
    let water = field.water;
    let lava = field.lava;
    let silt = field.silt;
    var colour = mix(vec3<f32>(0.012, 0.023, 0.029), vec3<f32>(0.035, 0.046, 0.050), silt);

    // Each moving layer is drawn only where its seam shows: away from the
    // veins `water` and `lava` are exactly zero, the mixes below would keep
    // `colour` as it is, and most of the cloth is away from the veins
    // (`docs/perf-client.md`).
    // Advected ripples, refracted caustics and narrow reflected crests.
    if water > 0.0 {
        let flow = vec2<f32>(warp.x * 2.4, warp.y * 1.4 - t * 0.34);
        let current = fbm(flow);
        let ripple = sin(flow.y * 18.0 + sin(flow.x * 4.0 + current * 8.0) * 0.8);
        let crest = pow(max(ripple, 0.0), 14.0);
        let depth = (1.0 - smoothstep(0.0, 1.3, water_d));
        var river = mix(vec3<f32>(0.018, 0.12, 0.16), vec3<f32>(0.012, 0.047, 0.075), depth);
        river += vec3<f32>(0.08, 0.27, 0.31) * current * 0.30;
        river += vec3<f32>(0.32, 0.52, 0.55) * crest * 0.065;
        let foam = smoothstep(0.52, 0.76, current) * (1.0 - depth) * 0.20;
        river += vec3<f32>(0.38, 0.49, 0.46) * foam;
        colour = mix(colour, river, water);
    }

    // Slower molten flow carries dark crust islands over glowing seams.
    if lava > 0.0 {
        let molten_uv = vec2<f32>(warp.x * 4.5, warp.y * 2.5 - t * 0.23);
        let crust = fbm(molten_uv);
        let crack = 1.0 - smoothstep(0.008, 0.09, abs(crust - 0.49));
        let core = 1.0 - smoothstep(0.1, 1.0, lava_d);
        var molten = mix(vec3<f32>(0.10, 0.023, 0.006), vec3<f32>(0.30, 0.075, 0.012), crack);
        molten += vec3<f32>(0.19, 0.11, 0.026) * crack * core;
        // Cooling at the confluence forms black glass and a thin pale steam veil.
        let contact = water * lava;
        molten = mix(molten, vec3<f32>(0.018, 0.026, 0.031), contact * (0.65 + crust * 0.25));
        colour = mix(colour, molten, lava);
        if contact > 0.0 {
            let steam = vnoise(vec2<f32>(p.x * 1.8 + t * 0.05, p.y * 1.2 - t * 0.16));
            colour += vec3<f32>(0.11, 0.15, 0.16) * contact * smoothstep(0.40, 0.80, steam) * 0.25;
        }
    }
    // The broad reflection belongs to the glass above both rivers.
    let reflection = exp(-pow((p.y + p.x * 0.28 + 2.5) * 0.23, 2.0));
    return colour + vec3<f32>(0.035, 0.052, 0.06) * reflection;
}

// Five enamel stones set into the compass. Mana still controls their light,
// but their silhouettes belong to the table. No layered noise, flame erosion,
// or screen-facing billboards in this small ornament.
const FLAME_REACH: f32 = 1.6;
const STILL_AT: f32 = 2.3;

fn inlay_stone(i: u32) -> vec3<f32> {
    if i == 0u { return vec3<f32>(0.85, 0.78, 0.58); }
    if i == 1u { return vec3<f32>(0.32, 0.57, 0.72); }
    if i == 2u { return vec3<f32>(0.51, 0.40, 0.59); }
    if i == 3u { return vec3<f32>(0.73, 0.36, 0.25); }
    return vec3<f32>(0.35, 0.62, 0.46);
}

// The one stone nearest `p`, found by its bearing rather than by a loop over
// all five: a stone's light reaches 0.36 and two stones stand 1.03 apart on
// their circle, so no pixel is lit by two. Outside the stones' band nothing
// is done at all. All in the dial's own units (`p` = table ÷ scale).
fn firewheel(start: vec3<f32>, p_dial: vec2<f32>, pixel: f32, t: f32) -> vec3<f32> {
    let r = length(p_dial);
    if r < STONE_R - 0.37 || r > STONE_R + 0.37 {
        return start;
    }
    let k = i32(round((1.5707964 - atan2(p_dial.y, p_dial.x)) * 5.0 / TAU));
    let i = u32(((k % 5) + 5) % 5);
    let angle = 1.5707964 - TAU * f32(k) / 5.0;
    let axis = vec2<f32>(cos(angle), sin(angle));
    let tangent = vec2<f32>(-axis.y, axis.x);
    let p = p_dial - axis * STONE_R;
    // A fifth smaller than at the old feet, so a stone stands between
    // the hub plate (0.72) and the outer gold ring (1.03) and on neither.
    let diamond = (abs(dot(p, tangent)) * 0.85 + abs(dot(p, axis)) * 0.58) / STONE_SCALE;
    var strength = params.flames_tail.x;
    if i < 4u { strength = params.flames[i]; }
    let breath = 0.72 + 0.28 * sin(t * 1.25 + f32(i) * 1.7);
    let body = 1.0 - smoothstep(0.105 - pixel, 0.105 + pixel, diamond);
    let rim = hairline(diamond - 0.12, 0.009, pixel);
    let facet = smoothstep(-0.13, 0.13, dot(p, axis + tangent));
    let tint = to_linear(inlay_stone(i));
    var lit = mix(start, tint * (0.48 + 0.50 * facet + 0.50 * strength) * breath, body);
    lit = mix(lit, to_linear(GILT) * 0.55, rim * 0.7);
    let glow = 1.0 - smoothstep(0.06, 0.36, length(p));
    return lit + tint * glow * glow * (0.12 + 0.30 * strength) * breath;
}

// The bezel the jewels stand in, cut into the cloth before the room's light
// reaches it (so it is lit like the table it is part of): the whole face a
// shade of smoked glass deeper than the cloth, a darker chapter band from
// `BAND_IN` to the rim with sixty engraved ticks on its inner edge, the
// compass hairline the jewels sit on, and a fine gilt rim.
fn bezel(cloth: vec3<f32>, p: vec2<f32>, r: f32, pixel: f32) -> vec3<f32> {
    let inside = 1.0 - smoothstep(BEZEL_R - pixel, BEZEL_R + pixel, r);
    let band = smoothstep(BAND_IN - pixel, BAND_IN + pixel, r) * inside;
    var colour = cloth * mix(1.0, 0.62, inside) * mix(1.0, 0.72, band);
    // Sixty ticks, every fifth longer: ornament, never a seat.
    if r > BAND_IN && r < BAND_IN + 0.06 {
        let turns = atan2(p.y, p.x) / TAU * 60.0;
        let across = abs(fract(turns + 0.5) - 0.5) * TAU * r / 60.0;
        let nearest = i32(round(turns));
        let fifth = select(0.0, 1.0, ((nearest % 5) + 5) % 5 == 0);
        let reach = mix(0.028, 0.050, fifth);
        let tick = (1.0 - smoothstep(0.0045, 0.0045 + pixel, across))
            * (1.0 - smoothstep(BAND_IN + reach - pixel, BAND_IN + reach + pixel, r));
        colour = mix(colour, ENGRAVING, tick * mix(0.30, 0.55, fifth));
    }
    colour = mix(colour, ENGRAVING, hairline(r - BAND_IN, 0.006, pixel) * 0.45);
    colour = mix(colour, ENGRAVING, hairline(r - COMPASS_R, 0.008, pixel) * 0.30);
    colour = mix(colour, GILT, hairline(r - BEZEL_R, 0.010, pixel) * 0.70);
    return colour;
}

// How far `p` lies outside a hand pointing along `dir` from the middle,
// `tip` long, `root` half-wide at the pivot and `end` half-wide at the tip;
// negative inside. A capsule-free taper: two hands per pixel, no loop.
fn hand_distance(p: vec2<f32>, dir: vec2<f32>, tip: f32, root: f32, end: f32) -> f32 {
    let along = dot(p, dir);
    let across = abs(p.x * dir.y - p.y * dir.x);
    let half = mix(root, end, clamp(along / max(tip, 1e-3), 0.0, 1.0));
    return max(max(across - half, along - tip), -along);
}

// The turn hand in its own frame (`along` from the pivot toward the seat,
// `across` either side), negative inside: a leaf-shaped blade that swells
// past the plate and narrows to a neck, an ivory lozenge at its tip, and
// on the far side of the pivot a counterweight — a short tail ending in a
// ring, long enough to show past the hub plate.
fn turn_shape(along: f32, across: f32, tip: f32) -> f32 {
    let neck = tip - 0.15;
    let belly = mix(HUB_R, neck, 0.30);
    var half = mix(0.068, 0.092, smoothstep(HUB_R - 0.2, belly, along));
    half = mix(half, 0.020, smoothstep(belly, neck, along));
    let blade = max(max(across - half, along - neck - 0.01), -along);
    let gem_at = tip - 0.078;
    let gem = (abs(along - gem_at) / 0.082 + across / 0.056 - 1.0) * 0.045;
    let bar = max(max(across - 0.024, along + 0.01), -along - TAIL_END + 0.05);
    let ring = abs(length(vec2<f32>(along + TAIL_END, across)) - 0.050) - 0.017;
    return min(min(blade, gem), min(bar, ring));
}

// The priority hand: a straight bar and an arrowhead at its tip.
fn prio_shape(along: f32, across: f32, tip: f32) -> f32 {
    let base = tip - 0.20;
    let bar = max(max(across - 0.030, along - base - 0.02), -along);
    let head = max(across - 0.080 * (tip - along) / 0.20, base - along);
    return min(bar, head);
}

// A light that rose at `at` and fades over 0.6 s, or the resting 0.35 when
// the table holds still.
fn arrival(at: f32) -> f32 {
    if params.motion < 0.5 {
        return 0.35;
    }
    let age = globals.time - at;
    if age < 0.0 || age > 0.6 {
        return 0.35;
    }
    return mix(1.0, 0.35, smoothstep(0.0, 0.6, age));
}

// The flare an arrival throws: a ring opening from the tip and going out
// over the same 0.6 s, nothing at all when the table holds still.
fn flare(p: vec2<f32>, centre: vec2<f32>, at: f32, pixel: f32) -> f32 {
    if params.motion < 0.5 {
        return 0.0;
    }
    let age = globals.time - at;
    if age < 0.0 || age > 0.6 {
        return 0.0;
    }
    let u = age / 0.6;
    let reach = 0.05 + 0.32 * (1.0 - (1.0 - u) * (1.0 - u));
    let ring = 1.0 - smoothstep(0.012, 0.012 + 2.0 * pixel, abs(length(p - centre) - reach));
    return ring * (1.0 - u) * (1.0 - u);
}

// The faint fan a sweeping hand leaves behind it: the arc between where it
// points and where it pointed a moment ago (`back`, `Hand::trail`), fading
// toward the old end; nothing while the two agree.
fn trail(p: vec2<f32>, r: f32, dir: vec2<f32>, back: vec2<f32>, tip: f32) -> f32 {
    let span = atan2(dir.x * back.y - dir.y * back.x, dot(dir, back));
    if abs(span) < 0.01 || r < HUB_R || r > tip {
        return 0.0;
    }
    let f = atan2(dir.x * p.y - dir.y * p.x, dot(dir, p)) / span;
    if f <= 0.0 || f >= 1.0 {
        return 0.0;
    }
    let reach = 1.0 - smoothstep(tip - 0.10, tip, r);
    return (1.0 - f) * (1.0 - f) * reach;
}

// One hand drawn: its shadow on the cloth, the dark outline that keeps it
// legible over a stone, its body bevelled along its spine (the half facing
// screen-up lit), and — while the table moves — a highlight travelling out
// along it every few seconds. Only what lies outside the hub plate.
fn paint_hand(
    lit_in: vec3<f32>,
    d: f32,
    d_shade: f32,
    side: f32,
    tone: vec3<f32>,
    shimmer: f32,
    outside: f32,
    pixel: f32,
) -> vec3<f32> {
    var lit = lit_in;
    let shade = (1.0 - smoothstep(-0.01, 0.035, d_shade)) * outside;
    lit = lit * (1.0 - 0.45 * shade);
    let edge = (1.0 - smoothstep(OUTLINE - pixel, OUTLINE + pixel, d)) * outside;
    let body = (1.0 - smoothstep(-pixel, pixel, d)) * outside;
    lit = mix(lit, to_linear(ENGRAVING) * 0.22, edge);
    let bevel = 0.86 + 0.16 * side;
    lit = mix(lit, tone * bevel + tone * shimmer * 0.55, body);
    return lit;
}

// The dial (DESIGN-v7 §3, dial-v2): one jewel per seat on the compass, the
// hub plate the turn number stands on, and two hands — the turn hand (ivory,
// a leaf blade with a lozenge at its tip and a counterweight) at the active
// seat and the priority hand (teal, a bar with an arrowhead) at the seat the
// table waits for. Drawn in the dial's own units — `p` is the table point
// over the dial's scale, `pixel` likewise — so the face is one drawing at
// every size. `t` is the still-able clock (`STILL_AT` when the table holds
// still), and every continuous light is also multiplied by `live`, so a
// still table draws the same frame twice. Only inside the face's reach, so
// the rest of the slab pays nothing; uniforms only.
fn clock_face(start: vec3<f32>, p: vec2<f32>, radius: f32, pixel: f32, t: f32) -> vec3<f32> {
    var lit = start;
    let live = step(0.5, params.motion);
    let up = params.flames_tail.yz;
    let ink = to_linear(ENGRAVING) * 0.30;
    let turn_len = length(params.hands.xy);
    let turn_dir = params.hands.xy / max(turn_len, 1e-4);
    let prio_len = params.dial.x;
    let prio_dir = normalize(params.hands.zw + vec2<f32>(0.0, 1e-6));
    let breath = mix(1.0, 0.82 + 0.18 * sin(t * 1.9), live);

    // The jewels, out on the band: a cut stone in the seat's colour, a dark
    // setting, a team's ring, and a halo in a hand's colour while that hand
    // rests on it. A seat choosing its opening hand wears a teal arc.
    if radius > 1.08 {
        for (var i = 0u; i < 8u; i = i + 1u) {
            if f32(i) >= params.seats {
                break;
            }
            let pair = params.jewels[i / 2u];
            var dir = pair.xy;
            if (i % 2u) == 1u {
                dir = pair.zw;
            }
            let tint = params.tints[i];
            let state = tint.a;
            if state >= 2.0 {
                let off = abs(atan2(dir.x * p.y - dir.y * p.x, dot(dir, p)));
                let arc = (1.0 - smoothstep(0.17, 0.18 + pixel, off))
                    * (1.0 - smoothstep(0.03 - pixel, 0.03 + pixel, abs(radius - 1.13)));
                lit = mix(lit, to_linear(TEAL), arc * 0.9);
            }
            let rel = p - dir * COMPASS_R;
            let d = length(rel);
            if d > 0.13 {
                continue;
            }
            var colour = to_linear(tint.rgb);
            if fract(state) > 0.01 {
                colour = mix(to_linear(ENGRAVING), colour, 0.25) * 0.5;
            }
            let on_turn = smoothstep(0.990, 0.9995, dot(dir, turn_dir)) * min(turn_len, 1.0);
            let on_prio = smoothstep(0.990, 0.9995, dot(dir, prio_dir)) * prio_len;
            let halo = exp(-pow((d - 0.092) / 0.018, 2.0)) * breath;
            lit += to_linear(IVORY) * halo * on_turn * 0.55 * arrival(params.dial.z) * 2.0;
            lit += to_linear(TEAL) * halo * on_prio * 0.55 * arrival(params.dial.w) * 2.0;
            let team = params.teams[i];
            let ring = 1.0 - smoothstep(0.007 - pixel, 0.007 + pixel, abs(d - 0.086));
            lit = mix(lit, to_linear(team.rgb), ring * team.a);
            let setting = 1.0 - smoothstep(JEWEL_R + 0.012 - pixel, JEWEL_R + 0.012 + pixel, d);
            lit = mix(lit, ink, setting);
            let body = 1.0 - smoothstep(JEWEL_R - pixel, JEWEL_R + pixel, d);
            let cut = 0.62 + 0.40 * (1.0 - d / JEWEL_R) + 0.22 * dot(rel / max(d, 1e-4), up);
            let glint = exp(-dot(rel - up * 0.024, rel - up * 0.024) / 0.00018);
            lit = mix(lit, colour * cut + vec3<f32>(glint * 0.55), body);
        }
    }

    // The hands, drawn beneath the hub plate: only what lies outside it
    // shows, so the number on the plate is never crossed. The priority hand
    // is drawn over the turn hand, its teal bar inside the wider blade when
    // both point at one seat.
    let outside = smoothstep(HUB_R - pixel, HUB_R + pixel, radius);
    let drop = up * 0.032;
    if turn_len > 0.01 {
        let tip = HUB_R + (TURN_TIP - HUB_R) * min(turn_len, 1.0);
        let back = select(turn_dir, normalize(params.trail.xy), length(params.trail.xy) > 0.5);
        lit = mix(lit, to_linear(IVORY) * 0.85, trail(p, radius, turn_dir, back, tip) * 0.40);
        let along = dot(p, turn_dir);
        let cross = p.x * turn_dir.y - p.y * turn_dir.x;
        let d = turn_shape(along, abs(cross), tip);
        let q = p + drop;
        let d_shade = turn_shape(dot(q, turn_dir), abs(q.x * turn_dir.y - q.y * turn_dir.x), tip);
        let side = sign(cross) * (turn_dir.y * up.x - turn_dir.x * up.y);
        let run = fract(t * 0.19);
        let shine = exp(-pow((along - mix(HUB_R, tip, run)) / 0.05, 2.0))
            * sin(run * 3.1415927) * live;
        // The counterweight is mechanism, not a pointer: gilt, so it never
        // reads as a hand on the seat across.
        let tone = mix(to_linear(IVORY), to_linear(GILT) * 0.75, step(along, 0.0));
        lit = paint_hand(lit, d, d_shade, side, tone, shine, outside, pixel);
        let gem = turn_dir * (tip - 0.078);
        let pool = 1.0 - smoothstep(0.0, 0.17, length(p - gem));
        lit += to_linear(IVORY) * pool * pool * 0.40 * arrival(params.dial.z) * breath;
        lit += to_linear(IVORY) * flare(p, gem, params.dial.z, pixel) * 0.9 * outside;
    }
    if prio_len > 0.01 {
        let tip = HUB_R + (PRIO_TIP - HUB_R) * prio_len;
        let back = select(prio_dir, normalize(params.trail.zw), length(params.trail.zw) > 0.5);
        lit = mix(lit, to_linear(TEAL) * 0.85, trail(p, radius, prio_dir, back, tip) * 0.40);
        let along = dot(p, prio_dir);
        let cross = p.x * prio_dir.y - p.y * prio_dir.x;
        let d = prio_shape(along, abs(cross), tip);
        let q = p + drop;
        let d_shade = prio_shape(dot(q, prio_dir), abs(q.x * prio_dir.y - q.y * prio_dir.x), tip);
        let side = sign(cross) * (prio_dir.y * up.x - prio_dir.x * up.y);
        let run = fract(t * 0.19 + 0.5);
        let shine = exp(-pow((along - mix(HUB_R, tip, run)) / 0.05, 2.0))
            * sin(run * 3.1415927) * live;
        lit = paint_hand(lit, d, d_shade, side, to_linear(TEAL), shine, outside, pixel);
        let head = prio_dir * (tip - 0.10);
        let pool = 1.0 - smoothstep(0.0, 0.17, length(p - head));
        lit += to_linear(TEAL) * pool * pool * 0.40 * arrival(params.dial.w) * prio_len * breath;
        lit += to_linear(TEAL) * flare(p, head, params.dial.w, pixel) * 0.9 * prio_len * outside;
        // A spark circling the arrowhead, a short tail of light behind it.
        let around = p - head;
        let off = length(around);
        if live > 0.5 && off < 0.16 {
            let spin = t * 2.4;
            let at = vec2<f32>(cos(spin), sin(spin)) * ORBIT;
            let spark = exp(-dot(around - at, around - at) / 0.00016);
            let behind = fract((spin - atan2(around.y, around.x)) / TAU);
            let band = exp(-pow((off - ORBIT) / 0.010, 2.0));
            let tail = exp(-behind * 9.0) * band * 0.5;
            lit += mix(to_linear(TEAL), vec3<f32>(1.0), 0.45) * (spark + tail) * prio_len;
        }
    }

    // The hub plate: the cloth darkened the way the black body is, a little
    // lighter toward its screen-up rim as a domed cap would be, an engraved
    // inner ring, and a gilt hairline at its edge that pulses once — and
    // throws a ripple out over the face — when a hand arrives at me.
    let plate = 1.0 - smoothstep(HUB_R - pixel, HUB_R + pixel, radius);
    let dome = smoothstep(HUB_R * 0.6, HUB_R, radius)
        * (0.5 + 0.5 * dot(p / max(radius, 1e-4), up));
    lit = mix(lit, lit * 0.22 + to_linear(GILT) * dome * 0.035, plate);
    lit = mix(lit, ink, hairline(radius - HUB_R * 0.925, 0.004, pixel) * 0.6);
    var rim_colour = to_linear(GILT) * 0.8;
    var rim_gain = 0.75;
    if params.pulse.y > 0.5 && params.motion > 0.5 {
        let age = globals.time - params.pulse.x;
        if age >= 0.0 && age <= 0.6 {
            let swell = sin(age / 0.6 * 3.1415927) * 0.8;
            var light = to_linear(IVORY);
            if params.pulse.y > 1.5 {
                light = to_linear(TEAL);
            }
            rim_colour = mix(rim_colour, light, swell);
            rim_gain = rim_gain + swell * 0.25;
            lit += light * plate * swell * 0.03;
            let u = age / 0.6;
            let wave = hairline(radius - (HUB_R + 0.45 * u), 0.012, pixel) * (1.0 - u) * (1.0 - u);
            lit += light * wave * 0.7;
        }
    }
    lit = mix(lit, rim_colour, hairline(radius - HUB_R, 0.011, pixel) * rim_gain);
    return lit;
}

/// The tear's line at `x` (`transition::tear_line`, the same arithmetic on
/// the same value noise): slow chunks, a middle swing, sharp creases at
/// uneven spacing, a grain. The pieces are cut along it on the CPU; here it only
/// says where the cut meets the veins.
fn tear_line(x: f32, seed: f32) -> f32 {
    let chunk = (vnoise(vec2<f32>(x * 0.11, seed * 1.3)) - 0.5) * 1.0;
    let swing = (vnoise(vec2<f32>(x * 0.47, seed + 3.3)) - 0.5) * 0.5;
    let crease = (abs(vnoise(vec2<f32>(x * 1.6, seed + 4.1)) * 2.0 - 1.0) - 0.5) * 0.36;
    let grain = (vnoise(vec2<f32>(x * 5.3, seed + 2.3)) - 0.5) * 0.08;
    return chunk + swing + crease + grain;
}

/// The apron, lit: the wall of the slab at `table`, `drop` of the way down
/// it, `faces` how much it turns toward the near edge.
fn apron_lit(table: vec2<f32>, drop: f32, faces: f32) -> vec3<f32> {
    // Down the height, darker as it goes: the underside of a table is in
    // its own shadow, and nothing here lights it.
    let grain = fbm(vec2<f32>(table.x + table.y, -drop * params.thickness * 6.0) * 2.0);
    let shade = mix(1.15, 0.42, drop) * mix(0.78, 1.0, faces);
    let trim = 1.0 - smoothstep(0.025, 0.09, abs(drop - 0.28));
    let apron = to_linear(APRON * shade + vec3<f32>((grain - 0.5) * 0.012) + ENGRAVING * trim * 0.22);
    return under_sky(under_lamp(apron, table));
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Table space out of the world position, rather than the mesh's own uv.
    // The uv origin is a convention of whichever builder made the mesh, and a
    // wrong guess about it mirrors the whole field — invisible at two seats,
    // because a duel is symmetric about both axes, and wrong at three.
    // `to_world` is `(x, height, -y)`, so this is exactly its inverse.
    // The tear's parts (`rift.w`, `table::pieces`): a piece's cut face, its
    // rim's wall and its seam each by their own branch, told apart by the
    // uv (a slab's never leaves 0 to 1), and the void by its own. The whole
    // slab (`w = 0`) takes none of them.
    var table = vec2<f32>(in.world_position.x, -in.world_position.z);
    // A piece of a tearing table, or the dial lifted off it, is drawn in its
    // own frame, out of the mesh's uv: the same table coordinates as the
    // world's while it stands where the slab does, and carried with it as it
    // slides, turns and lifts.
    if (params.rift.w > 0.5) {
        table = vec2<f32>((in.uv.x - 0.5) * params.span.x, (0.5 - in.uv.y) * params.span.y);
    }
    // Derivatives precede every return: a return under a per-pixel condition
    // (the piece's uv below) leaves the rest in non-uniform control flow,
    // where WebGPU refuses `fwidth` and the whole pipeline with it.
    let footprint = fwidth(table);
    let piece = params.rift.w > 0.5 && params.rift.w < 1.5;
    if (piece && in.uv.x > 1.5) {
        if (in.uv.x > 5.5) {
            return seam_strip(in.uv);
        }
        if (in.uv.x > 3.5) {
            return piece_wall(in.uv, in.world_normal);
        }
        return cut_face(in.uv);
    }
    if (params.rift.w > 1.5 && params.rift.w < 2.5) {
        return void_floor(in.uv);
    }
    let pixel = max(length(footprint), 0.001);
    let half = params.span * 0.5;

    // The apron: the wall of the slab. Told apart by its normal, which is the
    // only face that does not point up.
    if (in.world_normal.y < 0.5) {
        let drop = clamp(-in.world_position.y / max(params.thickness, 1e-3), 0.0, 1.0);
        // A little brighter on the near side, which is the one edge of a
        // table anybody ever sees at this camera.
        let faces = clamp(in.world_normal.z * 0.5 + 0.5, 0.0, 1.0);
        return vec4<f32>(apron_lit(table, drop, faces), 1.0);
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
        colour = glass_at(table) + vec3<f32>(crest * ROLL_LIGHT);
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

    // The dial is drawn in its own units: the table point over its scale
    // (`baylee_client_core::dial::scale_for`), so it grows with the free
    // middle of the table and every part of it with it.
    let radius = length(table);
    let scale = max(params.pulse.z, 1.0);
    let dial_r = radius / scale;
    let dial_p = table / scale;
    let dial_px = pixel / scale;
    // The bezel, cut into the cloth: low contrast and static — a spinning
    // bezel under two hands is a clock with a loose dial (DESIGN-v7 §3.2).
    if dial_r < BEZEL_R + 0.05 && inset > 0.0 {
        colour = bezel(colour, dial_p, dial_r, dial_px);
    }

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
    // Ring wear is only evaluated in its small footprint.
    var tarnish = 0.9;
    if dial_r < FLAME_REACH { tarnish = 0.80 + 0.20 * vnoise(dial_p * 6.0); }
    let rings = clamp(hairline(dial_r - 1.03, 0.017, dial_px) * 0.85, 0.0, 1.0);
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
    var lit = under_sky(under_lamp(to_linear(colour), table));

    // The firewheel, inside its own reach and nowhere else. The branch is
    // the budget as well as the shape: five flames cost nothing at all over
    // the other 99% of a thirty-five-unit slab.
    //
    // It is applied to *lit* cloth rather than to `colour`, which is not a
    // detail — a flame is something the table emits, so a white one would go
    // blue at night if it were graded by the sky like the cloth under it.
    if dial_r < FLAME_REACH && !piece {
        let t = mix(STILL_AT, globals.time, params.motion);
        lit = firewheel(lit, dial_p, dial_px, t);
        lit = clock_face(lit, dial_p, dial_r, dial_px, t);
    }
    // A piece of a tearing table: the dial has been lifted off it whole, and
    // what is left is its bed — the cloth sunk in shadow inside a gilt
    // hairline.
    if (piece && dial_r < DIAL_R + dial_px) {
        let bed = 1.0 - smoothstep(DIAL_R - dial_px, DIAL_R + dial_px, dial_r);
        lit = mix(lit, lit * 0.18, bed);
        lit = mix(lit, to_linear(GILT) * 0.5, hairline(dial_r - DIAL_R, 0.012, dial_px));
    }
    return vec4<f32>(lit + glow, 1.0);
}
