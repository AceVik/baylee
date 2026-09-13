// The air over the table: leaves, mist, embers, shafts, fog and flakes.
//
// One quad, cut to the slab's own racetrack and lying three and a half
// thousandths of a unit above the felt — above every mark the table draws and
// below the contact shadow under a card. The cards are opaque and write
// depth, so nothing here can reach a card pixel. That is the whole promise,
// and it is geometry rather than restraint.
//
// No texture, no sprite, no gradient: `docs/legal.md` §2 decided that for the
// felt and §5 for the sounds, and it is the same decision here. Everything
// below is hashes, noise and a handful of signed distances.
//
// # How a field of marks is drawn without a particle system
//
// A scrolling cell grid. Table space is divided into cells, one mark per
// cell at a hashed offset inside it, and the *grid* is what moves: the drift
// is subtracted from the coordinate before the floor, so every mark travels
// together and none of them is ever born or destroyed at a cell boundary.
// Three such grids at different cell sizes and speeds are a depth field, and
// the parallax between them is what says the marks are in the air rather than
// on the lens.
//
// # Compositing
//
// The material is `AlphaMode::Blend`, so what comes out is
// `rgb * a + felt * (1 - a)`: every layer lightens or darkens the cloth
// *towards* its own colour and can never blow past it. Each layer therefore
// contributes a premultiplied colour and a weight, and the sum is
// un-premultiplied at the end. A sheet that covers the whole table (mist,
// fog, shafts) has to be far quieter than a field of small marks, because its
// weight is paid on every pixel and theirs is paid on almost none.
//
// # WebGL2
//
// Uniforms only, no storage buffers, no texture arrays, every loop bound at
// compile time. The loops below count to three literally.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::globals

struct AtmosphereParams {
    /// The slab's world size, which turns a world position into a point on
    /// the table.
    span: vec2<f32>,
    /// The direction a falling thing appears to travel on the table plane:
    /// towards the camera's own ground position. Recomputed as the player
    /// orbits, which is what stops this reading as a filter on the lens.
    fall: vec2<f32>,
    /// Forest.
    leaves: f32,
    /// Island.
    mist: f32,
    /// Mountain.
    embers: f32,
    /// Plains.
    shafts: f32,
    /// Swamp.
    fog: f32,
    /// Snow.
    flakes: f32,
    /// The clock: 1 normally, 0 when the player asked for a still table. At 0
    /// the air freezes rather than emptying — a still table is the same room
    /// held still, not a different one.
    motion: f32,
    /// The corner radius the mesh was cut with, so the air can be faded out
    /// where the table stops instead of ending on a line.
    corner: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> params: AtmosphereParams;

// Colours, display-referred like every colour this project writes down. They
// are what each layer pulls the cloth *towards*, so a colour brighter than
// the baize lightens and a darker one weighs it down — which is the whole
// difference between an island's mist and a swamp's fog.
const LEAF: vec3<f32> = vec3<f32>(0.62, 0.44, 0.17);
const MIST: vec3<f32> = vec3<f32>(0.58, 0.76, 0.88);
const EMBER: vec3<f32> = vec3<f32>(1.00, 0.58, 0.20);
const SHAFT: vec3<f32> = vec3<f32>(1.00, 0.94, 0.74);
const FOG: vec3<f32> = vec3<f32>(0.16, 0.15, 0.19);
const FLAKE: vec3<f32> = vec3<f32>(0.86, 0.92, 1.00);

// How much of the cloth each layer may take at amplitude 1, as a blend
// weight. The three that are made of small marks may be strong where the mark
// is, because a mark is a fiftieth of a card wide and covers almost nothing;
// the three that are sheets are paid on every pixel of the table and are an
// order of magnitude quieter for exactly that reason.
const LEAF_WEIGHT: f32 = 0.30;
const FLAKE_WEIGHT: f32 = 0.34;
const EMBER_WEIGHT: f32 = 0.40;
const MIST_WEIGHT: f32 = 0.055;
const FOG_WEIGHT: f32 = 0.075;
const SHAFT_WEIGHT: f32 = 0.045;

/// The most of the cloth all six together may ever take.
///
/// They cannot reach it — the six amplitudes sum to at most one and the three
/// sheets are a twentieth each — but a cap is what makes that a property of
/// the shader rather than of the arithmetic upstream of it, and upstream is
/// where a future layer would be added.
const CEILING: f32 = 0.40;

/// How far in from the table's edge the air fades out, in table units.
///
/// A sheet that ended on the rail would draw a line round the table. One and
/// a half units is a little over a card's width, which is enough that the
/// fade is never seen as a fade.
const SHORE: f32 = 1.5;

const TAU: f32 = 6.2831855;

/// A hash with no trigonometry in it.
///
/// `sin`-based hashes differ between drivers — the same table would grain
/// differently on two machines, which is the argument `shaders/ambience.wgsl`
/// already made and the felt's own noise predates.
fn hash21(p: vec2<f32>) -> f32 {
    var h = dot(p, vec2<f32>(127.1, 311.7));
    h = fract(h * 0.1031);
    h *= h + 33.33;
    h *= h + h;
    return fract(h);
}

/// Two independent hashes of the same cell: where the mark sits in it, and
/// which mark it is.
fn hash22(p: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(hash21(p), hash21(p + vec2<f32>(41.7, 19.3)));
}

/// Value noise, quintic-smoothed so the field has no visible cell edges.
fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

/// Three octaves. A fourth is not visible at these weights and the sheets are
/// the only callers.
fn fbm(p: vec2<f32>) -> f32 {
    var sum = 0.0;
    var amp = 0.5;
    var at = p;
    for (var i = 0; i < 3; i = i + 1) {
        sum = sum + amp * noise(at);
        at = at * 2.03 + vec2<f32>(17.0, 9.0);
        amp = amp * 0.5;
    }
    return sum / 0.875;
}

/// A rounded rectangle's signed distance: negative inside, positive outside.
fn sd_round_box(p: vec2<f32>, half: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - half + vec2<f32>(r);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

/// One scrolling grid of round marks.
///
/// `cell` is the grid's pitch in table units, `radius` how big a mark is as a
/// fraction of a cell, `drift` how far the whole grid has travelled. The mark
/// sits at a hashed point *inside* its cell and the grid is what moves, so a
/// mark is never born at a boundary and never dies at one — which is the
/// thing a naive per-cell lifetime gets wrong and which reads as popping.
///
/// Returns coverage from 0 to 1 and, in `y`, the cell's own hash, so a caller
/// can give each mark its own size, flicker or phase.
fn field(p: vec2<f32>, cell: f32, radius: f32, drift: vec2<f32>, seed: f32) -> vec2<f32> {
    let q = (p - drift) / cell + vec2<f32>(seed);
    let id = floor(q);
    let h = hash22(id);
    // Kept off the cell walls, so two marks in neighbouring cells cannot
    // touch and read as one longer thing.
    let at = vec2<f32>(0.25, 0.25) + h * 0.5;
    let d = length(fract(q) - at);
    // Size varies with the cell, which is what stops a grid reading as a
    // grid: an even field of identical dots is the one thing that gives it
    // away at a glance.
    let r = radius * (0.6 + 0.8 * h.x);
    return vec2<f32>(1.0 - smoothstep(r * 0.35, r, d), h.y);
}

/// The same, as an ellipse that turns over as it travels.
///
/// A leaf is not a dot: what says *leaf* at this distance is that the thing
/// narrows and widens as it falls, and that costs one cosine. `spin` is how
/// far through its turn the mark is.
fn leaf_field(p: vec2<f32>, cell: f32, radius: f32, drift: vec2<f32>, seed: f32, t: f32) -> vec2<f32> {
    let q = (p - drift) / cell + vec2<f32>(seed);
    let id = floor(q);
    let h = hash22(id);
    let at = vec2<f32>(0.25, 0.25) + h * 0.5;
    var d = fract(q) - at;
    // Its own heading, so a drift of leaves is not a shoal of fish.
    let a = h.x * TAU;
    let turned = vec2<f32>(d.x * cos(a) - d.y * sin(a), d.x * sin(a) + d.y * cos(a));
    // Edge-on twice a turn: the narrow axis is a cosine that passes through
    // zero, and the wide one holds.
    let spin = 0.25 + 0.75 * abs(cos(t * (0.7 + h.y * 0.9) + h.x * TAU));
    let r = radius * (0.6 + 0.8 * h.x);
    let shaped = vec2<f32>(turned.x / max(spin, 0.08), turned.y * 1.45);
    let dist = length(shaped);
    return vec2<f32>(1.0 - smoothstep(r * 0.4, r, dist), h.y);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Table space out of the world position, exactly as the felt does it:
    // `to_world` is `(x, height, -y)`, so this is its inverse. The mesh's own
    // uv is a convention of whichever builder cut it, and a wrong guess
    // mirrors the whole field.
    let table = vec2<f32>(in.world_position.x, -in.world_position.z);
    let t = globals.time * params.motion;
    let fall = params.fall;
    // The one direction across the fall, for wobble and for the shafts.
    let side = vec2<f32>(-fall.y, fall.x);

    // Premultiplied colour and the weight it was multiplied by.
    var acc = vec3<f32>(0.0);
    var weight = 0.0;

    // Forest. Three depths: the far one small, slow and dense, the near one
    // large, fast and sparse. The parallax between them is the whole of the
    // claim that this is air and not a lens.
    if (params.leaves > 0.0) {
        var cover = 0.0;
        cover += leaf_field(table, 2.6, 0.20, fall * (t * 0.55), 0.0, t).x * 0.55;
        cover += leaf_field(table, 3.4, 0.26, fall * (t * 0.85), 7.3, t).x * 0.75;
        cover += leaf_field(table, 4.6, 0.34, fall * (t * 1.25), 19.1, t).x * 1.00;
        let w = min(cover, 1.0) * params.leaves * LEAF_WEIGHT;
        acc += LEAF * w;
        weight += w;
    }

    // Snow. Smaller, many more, and slower than anything else in the air,
    // with a lateral wobble so it settles rather than rains.
    if (params.flakes > 0.0) {
        var cover = 0.0;
        for (var i = 0; i < 3; i = i + 1) {
            let k = f32(i);
            let speed = 0.22 + k * 0.16;
            let sway = side * sin(t * (0.6 + k * 0.25)) * (0.10 + k * 0.06);
            let cell = 1.25 + k * 0.55;
            let mark = field(table, cell, 0.085 + k * 0.030, fall * (t * speed) + sway, k * 13.7);
            cover += mark.x * (0.55 + k * 0.22);
        }
        let w = min(cover, 1.0) * params.flakes * FLAKE_WEIGHT;
        acc += FLAKE * w;
        weight += w;
    }

    // Mountain. Rising — against the fall — small, and going out: each ember
    // has a life of its own and spends it fading in, flickering and dying.
    if (params.embers > 0.0) {
        var cover = 0.0;
        for (var i = 0; i < 2; i = i + 1) {
            let k = f32(i);
            let speed = 0.45 + k * 0.35;
            let drift = -fall * (t * speed) + side * sin(t * (0.9 + k * 0.5)) * 0.12;
            let mark = field(table, 2.9 + k * 1.4, 0.055 + k * 0.025, drift, 31.0 + k * 5.0);
            // A life: in, then out, over about four and a half seconds, each
            // ember started at its own point in it.
            let age = fract(t * 0.22 + mark.y);
            let alive = smoothstep(0.0, 0.15, age) * (1.0 - smoothstep(0.45, 1.0, age));
            let flicker = 0.70 + 0.30 * sin(t * 11.0 + mark.y * 40.0);
            cover += mark.x * alive * flicker * (0.7 + k * 0.5);
        }
        let w = min(cover, 1.0) * params.embers * EMBER_WEIGHT;
        acc += EMBER * w;
        weight += w;
    }

    // Island. A sheet rather than marks: low-frequency noise warped by a
    // slower copy of itself, which is what stops fbm from reading as a stain
    // and starts it reading as something moving over the table.
    if (params.mist > 0.0) {
        let creep = side * (t * 0.09) + fall * (t * 0.05);
        let warp = fbm(table * 0.12 + creep * 0.4);
        let sheet = fbm(table * 0.22 + creep + vec2<f32>(warp * 1.6));
        // Centred and steepened: mist is patchy, and a field that never
        // reaches zero is a filter over the whole table.
        let body = smoothstep(0.42, 0.88, sheet);
        let w = body * params.mist * MIST_WEIGHT;
        acc += MIST * w;
        weight += w;
    }

    // Swamp. The same construction, half the speed and a third of the
    // frequency — and a colour *darker* than the baize, so it lies in the
    // table rather than drifting over it.
    if (params.fog > 0.0) {
        let creep = side * (t * 0.035) - fall * (t * 0.02);
        let warp = fbm(table * 0.07 + creep * 0.5 + 11.0);
        let sheet = fbm(table * 0.13 + creep + vec2<f32>(warp * 2.1) + 4.0);
        let body = smoothstep(0.34, 0.84, sheet);
        let w = body * params.fog * FOG_WEIGHT;
        acc += FOG * w;
        weight += w;
    }

    // Plains. Broad shafts laid across the table, crossing the fall rather
    // than following it, travelling slowly enough that a player reading a
    // card never catches one moving. Two frequencies with no common multiple,
    // so the pattern does not repeat while anybody is looking at it.
    if (params.shafts > 0.0) {
        let along = dot(table, side) + t * 0.22;
        let across = dot(table, fall);
        // A narrow bright core with a wide gap: the eighth power is what
        // turns a sine into a shaft.
        let band = pow(max(sin(along * 0.42 + fbm(table * 0.06) * 2.0), 0.0), 8.0);
        let second = pow(max(sin(along * 0.27 - 1.1 + fbm(table * 0.05 + 3.0) * 2.0), 0.0), 10.0);
        // Broken up along their length, so a shaft is light coming through
        // something and not a painted stripe.
        let dapple = 0.55 + 0.45 * fbm(vec2<f32>(across * 0.18, along * 0.05) + t * 0.01);
        let w = (band + second * 0.7) * dapple * params.shafts * SHAFT_WEIGHT;
        acc += SHAFT * w;
        weight += w;
    }

    // The table's edge. Everything fades out over the last stretch of felt so
    // no sheet ends on a line, and the racetrack is the same one the slab was
    // cut with.
    let inside = -sd_round_box(table, params.span * 0.5, params.corner);
    let shore = smoothstep(0.0, SHORE, inside);

    let total = min(weight, CEILING) * shore;
    if (total <= 0.0005) {
        // Nothing in the air here. Returning a transparent fragment rather
        // than discarding keeps the branch uniform, which is cheaper on
        // every backend that cares.
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
    // Un-premultiply: `acc` is the sum of colour × weight, so this is the
    // colour the cloth is being pulled towards, and `total` is how far.
    return vec4<f32>(acc / max(weight, 1e-4), total);
}
