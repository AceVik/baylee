// The table the game is played on: a slab of casino baize inside a padded
// leather rail, with an apron below it.
//
// No texture at all. The slab is about thirty-five units across and a card is
// roughly 114 physical pixels at this camera, so drawing the cloth sharply
// would want some four thousand texels — and measured in a debug build,
// generating 2048 already costs 1.6 seconds every time the table is re-cut.
// Arithmetic has no resolution.
//
// `baylee_client_core::tabletop::felt` remains the reference: it is the same
// arithmetic on the CPU, where a test can block the image at card size and
// measure that the tooth survives and that the cloth stays dark enough to
// read a card against. `table::shader_tests::the_shader_and_the_generator_
// agree_about_the_cloth` reads the six colours below out of this file and
// fails if the two drift apart.
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
const FELT_DEEP: vec3<f32> = vec3<f32>(0.024, 0.086, 0.058);
const FELT_CLOTH: vec3<f32> = vec3<f32>(0.071, 0.223, 0.150);
const FELT_WORN: vec3<f32> = vec3<f32>(0.100, 0.285, 0.196);
const RAIL_HIDE: vec3<f32> = vec3<f32>(0.115, 0.072, 0.058);
const RAIL_LIP: vec3<f32> = vec3<f32>(0.196, 0.130, 0.100);
const APRON: vec3<f32> = vec3<f32>(0.055, 0.038, 0.030);

const TAU: f32 = 6.2831855;

/// Threads per table unit. A card is one unit wide, so this is how many
/// threads cross a card: enough that the cloth has a tooth at reading
/// distance, few enough that it never turns into stripes.
const WEAVE: f32 = 5.5;

/// How far the rail's shadow reaches onto the cloth, in table units.
///
/// The one thing that says the rail is *raised* rather than painted on. There
/// is no light in this scene to cast it, so it is painted — which is the same
/// argument the cards' contact shadows make, and the same reason.
const RAIL_SHADOW: f32 = 1.8;

/// How far along the rail the crown of the padding sits, as a fraction of its
/// width. Not the middle: a rail is rolled over its inner edge, so the light
/// on it sits inboard of centre.
const CROWN: f32 = 0.42;

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

/// A rounded rectangle's signed distance: negative inside, positive outside.
fn sd_round_box(p: vec2<f32>, half: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - half + vec2<f32>(r);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

/// The cloth at a point of table, in display-referred colour.
fn baize_at(p: vec2<f32>) -> vec3<f32> {
    // Big soft blotches of wear, then a fine grain and the weave on top —
    // the same three layers the CPU reference mixes, at table frequencies
    // rather than texel ones.
    let wear = fbm2(p * 0.085 + 3.1);
    let grain = fbm2(p * 4.2 + 17.9);
    let weave = sin(p.x * WEAVE * TAU) * sin(p.y * WEAVE * TAU) * 0.5 + 0.5;

    var colour = mix(FELT_CLOTH, FELT_WORN, pow(wear, 1.6));
    let lift = (grain - 0.5) * 0.030 + (weave - 0.5) * 0.012;
    return colour + vec3<f32>(lift);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Table space out of the world position, rather than the mesh's own uv.
    // The uv origin is a convention of whichever builder made the mesh, and a
    // wrong guess about it mirrors the whole field — invisible at two seats,
    // because a duel is symmetric about both axes, and wrong at three.
    // `to_world` is `(x, height, -y)`, so this is exactly its inverse.
    let table = vec2<f32>(in.world_position.x, -in.world_position.z);
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
        return vec4<f32>(to_linear(APRON * shade + vec3<f32>((grain - 0.5) * 0.012)), 1.0);
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
        // The cloth, falling into the rail's shadow as it reaches the edge.
        let lit = smoothstep(0.0, RAIL_SHADOW, inset);
        colour = mix(FELT_DEEP, baize_at(table), lit);
        // A hair of the lamp spills off the rail onto the cloth beside it,
        // and no further. Without it the rail reads as a sticker.
        lamp_here = (1.0 - lit) * 0.35;
    } else {
        // The rail: a padded roll, crowned inboard of its own middle.
        let across = clamp(-inset / max(params.rail, 1e-3), 0.0, 1.0);
        let crown = max(1.0 - abs(across - CROWN) / CROWN, 0.0);
        let hide = fbm2(table * 7.0 + 41.3);
        colour = mix(RAIL_HIDE, RAIL_LIP, smoothstep(0.0, 1.0, crown))
            + vec3<f32>((hide - 0.5) * 0.022);
        lamp_here = 0.35 + 0.65 * crown;
    }

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
    return vec4<f32>(to_linear(colour) + glow, 1.0);
}
