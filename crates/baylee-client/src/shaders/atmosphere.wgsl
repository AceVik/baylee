// The air over the table: leaves, mist, embers, shafts, fog and flakes.
//
// One quad, cut to the slab's own racetrack and lying three and a half
// thousandths of a unit above the felt. The cards are opaque and write depth,
// so nothing here can reach a card pixel; the rest of the table's ground is
// kept underneath by the sort ladder in `table.rs`. That is the whole
// promise, and it is geometry rather than restraint.
//
// No texture, no sprite, no gradient: `docs/legal.md` §2 decided that for the
// felt and §5 for the sounds, and it is the same decision here. Everything
// below is hashes, noise and a handful of signed distances.
//
// # Two kinds of thing, and two currencies
//
// A **veil** (mist, fog, shafts) is paid for on every pixel of the table, so
// its loudness is a *mean added lightness* and it is measured in hundredths.
// A **mark** (a leaf, an ember, a flake) covers almost nothing, so it may be
// opaque where it is — and the amplitude scales how *many* there are rather
// than how solid each one is. Half a forest is half as many leaves, not twice
// as many ghosts, which is the difference between weather easing away and
// weather fading out.
//
// # How a field of marks is drawn without a particle system
//
// A hashed cell grid whose *domain* scrolls. Table space is divided into
// cells, one mark per cell at a hashed offset, and the drift is subtracted
// from the coordinate before the floor — so a mark lives in its cell forever
// and rides with it, and there is no boundary for it to pop across. The one
// discontinuity left is the re-hash when a mark's life wraps, and the life
// envelope is exactly zero there.
//
// Nothing folds the cell index back into a small range. The temptation is
// real — a cell index grows without bound and `hash2` loses resolution as it
// does — but every fold either re-rolls the whole field at once or leaves a
// seam, and the arithmetic says it is not needed: neighbouring cells differ
// by 32.1 in the hash's first product, and an `f32` still resolves that
// several thousand cells out, which is hours of drift.
//
// # Compositing
//
// Back to front, premultiplied, inside the shader, and the order is the order
// these things are in the air: veil, shafts, flakes, leaf shadows, leaves,
// embers. Every layer pulls the cloth *towards* its own colour and can never
// blow past it, which is what makes a stack of six safe.
//
// Colours here are display-referred like every colour this project writes
// down, and are run through `to_linear` before they are composited — the
// render target is linear, and a surface meant to sit at 0.22 measures 0.45
// on screen if that conversion is skipped.
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
    /// The corner radius the mesh was cut with.
    corner: f32,
    /// How wide the padded rail runs, so a veil can stop on the cloth.
    rail: f32,
    day: f32,
    budget: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> params: AtmosphereParams;

// Every colour a layer pulls the cloth towards, display-referred.
const LEAF_OCHRE: vec3<f32> = vec3<f32>(0.62, 0.42, 0.14);
const LEAF_RUST: vec3<f32> = vec3<f32>(0.55, 0.26, 0.10);
const LEAF_GOLD: vec3<f32> = vec3<f32>(0.52, 0.48, 0.14);
const MIST: vec3<f32> = vec3<f32>(0.55, 0.66, 0.70);
const FOG_BODY: vec3<f32> = vec3<f32>(0.17, 0.13, 0.21);
const FOG_CREST: vec3<f32> = vec3<f32>(0.40, 0.36, 0.46);
const SHAFT: vec3<f32> = vec3<f32>(1.00, 0.94, 0.78);
const FLAKE: vec3<f32> = vec3<f32>(0.82, 0.88, 0.95);
const EMBER_BIRTH: vec3<f32> = vec3<f32>(1.00, 0.72, 0.30);
const EMBER_MID: vec3<f32> = vec3<f32>(0.85, 0.30, 0.08);
const EMBER_ASH: vec3<f32> = vec3<f32>(0.35, 0.08, 0.03);

/// Peak alpha a shaft may reach on the cloth, at amplitude 1.
///
/// **Derived, not chosen.** `the_felt_is_dark_enough_to_read_cards_against`
/// bounds the felt's display luma below 0.30 and `FELT_WORN` already sits at
/// about 0.24, which leaves roughly 0.047 of linear headroom. The shaft's own
/// linear luma is 0.876 against the cloth's 0.039, so `0.039 + 0.837a` may
/// not pass 0.086 and `a` may not pass 0.031. Shafts are the only layer that
/// *lifts* the felt behind a card, and they are the one that must never be
/// allowed to compete with it.
const SHAFT_PEAK: f32 = 0.025;

/// Peak alpha for the two veils at amplitude 1.
const MIST_PEAK: f32 = 0.10;
const FOG_PEAK: f32 = 0.35;

/// How far inside the rail each veil stops, in table units.
///
/// Fog reaches a whole unit less far than mist, and that difference is one of
/// the three things that say fog is lying *in* the table rather than drifting
/// over it — the others being that it darkens instead of lifting, and that it
/// churns in place instead of travelling. Marks are not faded at all: a leaf
/// over the leather is in the air, and stopping it at the rail would be the
/// one detail that gave the whole thing away as a picture on the cloth.
const MIST_SHORE: f32 = 1.5;
const FOG_SHORE: f32 = 2.5;

const TAU: f32 = 6.2831855;

/// A hash with no trigonometry in it.
///
/// `sin`-based hashes differ between drivers — the same table would grain
/// differently on two machines, which is the argument `shaders/ambience.wgsl`
/// already made.
fn hash2(p: vec2<f32>) -> f32 {
    var h = dot(p, vec2<f32>(127.1, 311.7));
    h = fract(h * 0.1031);
    h *= h + 33.33;
    h *= h + h;
    return fract(h);
}

/// Value noise, quintic-smoothed so the field has no visible cell edges.
fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let a = hash2(i);
    let b = hash2(i + vec2<f32>(1.0, 0.0));
    let c = hash2(i + vec2<f32>(0.0, 1.0));
    let d = hash2(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

/// Three octaves, normalised to 0..1. A fourth is not visible at these
/// weights, and the veils are the only callers.
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

/// sRGB to linear, componentwise.
fn to_linear(c: vec3<f32>) -> vec3<f32> {
    let hi = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    let lo = c / 12.92;
    return select(lo, hi, c > vec3<f32>(0.04045));
}

/// A rounded rectangle's signed distance: negative inside, positive outside.
fn sd_round_box(p: vec2<f32>, half: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - half + vec2<f32>(r);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

/// One `over` of a straight-alpha layer onto a premultiplied accumulator.
fn over(acc: vec4<f32>, rgb: vec3<f32>, a: f32) -> vec4<f32> {
    let k = clamp(a, 0.0, 1.0) * (1.0 - acc.a);
    return vec4<f32>(acc.rgb + rgb * k, acc.a + k);
}

/// One mark of a scrolling cell grid.
struct Mark {
    /// Where this fragment is relative to the mark, in **table units**.
    at: vec2<f32>,
    /// Two hashes re-rolled once per life: everything the mark *is*.
    s1: f32,
    s2: f32,
    /// A third, fixed for the life of the cell: whether there is a mark here
    /// at all at this amplitude.
    keep: f32,
    /// How far through its life the mark is, 0 to 1.
    phase: f32,
    /// Fade in, hold, fade out. Exactly zero where the hashes re-roll.
    env: f32,
}

/// Finds the mark of grid `j` that this fragment belongs to.
fn mark_at(p: vec2<f32>, size: f32, travel: vec2<f32>, life: f32, j: f32, t: f32) -> Mark {
    let lane = 17.3 * (j + 1.0);
    let q = (p - travel) / size + vec2<f32>(lane);
    let id = floor(q);
    let seed = hash2(id + 7.3 * (j + 1.0));
    let clock = t / life + seed;
    let phase = fract(clock);
    let gen = floor(clock);
    let s1 = hash2(id + gen * 13.7 + 31.0 * (j + 1.0));
    let s2 = hash2(id + gen * 13.7 + 53.0 * (j + 1.0));
    // Kept inside the cell, so a mark never needs its neighbours looked up.
    let jitter = (vec2<f32>(s1, s2) - 0.5) * 0.6;
    return Mark(
        (fract(q) - vec2<f32>(0.5) - jitter) * size,
        s1,
        s2,
        hash2(id + 3.1),
        phase,
        smoothstep(0.0, 0.12, phase) * (1.0 - smoothstep(0.78, 1.0, phase)),
    );
}

/// Whether a cell carries a mark at this amplitude, softened so a mark fades
/// in as the weather eases up instead of appearing whole.
fn alive(m: Mark, keep: f32, amp: f32) -> f32 {
    return smoothstep(0.0, 0.12, keep * amp - m.keep);
}

/// Rotates a vector.
fn turn(v: vec2<f32>, a: f32) -> vec2<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec2<f32>(v.x * c - v.y * s, v.x * s + v.y * c);
}

/// The slab's half-extent along a direction.
fn reach_along(half: vec2<f32>, dir: vec2<f32>) -> f32 {
    return abs(half.x * dir.x) + abs(half.y * dir.y);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Table space out of the world position, exactly as the felt does it:
    // `to_world` is `(x, height, -y)`, so this is its inverse. The mesh's own
    // uv is a convention of whichever builder cut it, and a wrong guess
    // mirrors the whole field.
    let table = vec2<f32>(in.world_position.x, -in.world_position.z);
    let t = globals.time * params.motion;
    let half = params.span * 0.5;
    // A camera exactly overhead has no ground offset, and `fall` arrives as a
    // zero vector; nudging it keeps every `normalize` below finite.
    let fall = normalize(params.fall + vec2<f32>(0.0, -1e-3));
    let across = vec2<f32>(-fall.y, fall.x);
    // How far onto the cloth a point is, past the rail. Positive on felt.
    let inset = -(sd_round_box(table, half, params.corner) + params.rail);

    // One lateral drift, shared by every layer that moves. There is one
    // weather over this table, not six: a field whose layers each wandered on
    // their own reads as six effects running at once.
    let wind = across * (0.25 * sin(t * 0.5));

    var acc = vec4<f32>(0.0);

    // Light through the air follows the same eased clock as the sky. Broad
    // shafts and sparse motes stay below cards, with no additional draw pass.
    let shore = smoothstep(0.0, 1.0, inset);
    let diagonal = table.x * 0.72 + table.y * 0.46;
    let beam = pow(max(0.0, sin(diagonal * 0.48 + 0.15 * sin(t * 0.08))), 12.0);
    let source = mix(vec3<f32>(0.40, 0.59, 0.76), vec3<f32>(0.99, 0.85, 0.61), params.day);
    acc = over(acc, to_linear(source), beam * shore * params.budget * mix(0.012, 0.035, params.day));
    for (var layer = 0; layer < 2; layer = layer + 1) {
        let depth = f32(layer);
        let drift = vec2<f32>(0.045, 0.025) * t * (1.0 + depth * 0.6);
        let m = mark_at(table, 2.8, drift, 13.0 + depth * 4.0, 31.0 + depth, t);
        let d = length(m.at);
        let radius = mix(0.024, 0.042, depth);
        let mote = 1.0 - smoothstep(0.0, radius, d);
        let glimmer = 0.65 + 0.35 * sin(t * 0.7 + m.s1 * TAU);
        let a = mote * m.env * alive(m, 0.32, params.budget) * shore * glimmer * 0.38;
        acc = over(acc, to_linear(source), a);
    }

    // The shafts are computed first because the veil is lit by them, and
    // drawn second because they are above it.
    var shaft_field = 0.0;
    if (params.shafts > 0.002) {
        // About thirty-five degrees off the across-axis, which is where the
        // sky puts its sun.
        let dir = normalize(across + 0.7 * fall);
        let side = vec2<f32>(-dir.y, dir.x);
        let u = dot(table, side);
        let w = dot(table, dir);
        let reach_u = max(reach_along(half, side), 1.0);
        let reach_w = max(reach_along(half, dir), 1.0);
        let centres = vec3<f32>(-0.30, 0.04, 0.36) * reach_u;
        let widths = vec3<f32>(1.6, 2.4, 1.3);
        // Breathing on three periods with no common multiple worth noticing,
        // the shortest of them about seventy seconds: a player reading a card
        // must never catch the table moving.
        var gains = vec3<f32>(0.70, 1.00, 0.55);
        gains.x *= 0.85 + 0.15 * sin(t * 0.09);
        gains.y *= 0.85 + 0.15 * sin(t * 0.09 + 2.1);
        gains.z *= 0.85 + 0.15 * sin(t * 0.09 + 4.2);
        // The third shaft only on a board that is mostly plains.
        gains.z *= smoothstep(0.3, 0.7, params.shafts);
        var band = 0.0;
        for (var i = 0; i < 3; i = i + 1) {
            let e = (u - centres[i]) / widths[i];
            band += gains[i] * exp(-e * e);
        }
        // The shafts themselves never move. What travels is the dust in them,
        // along their own length, which is what light through a window does
        // and what a sliding stripe does not.
        let dust = 0.55 + 0.45 * fbm(vec2<f32>(u * 0.6, w * 0.12 - t * 0.05) + 9.7);
        let entering = 0.7 + 0.3 * smoothstep(-reach_w, reach_w, w);
        shaft_field = band * dust * entering;
    }

    // Mist and fog are one veil and not two. They occupy the same register —
    // an area wash over the whole table — and two of them stacked at half
    // strength each make a flat grey neither board asked for.
    let a_mist = veil_mist(table, t, wind, across, inset);
    let a_fog = veil_fog(table, t, half, inset);
    let veil_a = a_mist + a_fog;
    if (veil_a > 0.0005) {
        let field = fog_field(table, t);
        let fog_rgb = mix(FOG_BODY, FOG_CREST, smoothstep(0.7, 0.95, field));
        var rgb = mix(fog_rgb, MIST, a_mist / veil_a);
        // Light through fog: the one cross-layer effect worth having, and the
        // reason the shafts are computed before the veil is drawn.
        rgb *= 1.0 + 1.5 * shaft_field * params.shafts;
        acc = over(acc, to_linear(rgb), veil_a);
    }

    if (params.shafts > 0.002) {
        acc = over(acc, to_linear(SHAFT), shaft_field * params.shafts * SHAFT_PEAK);
    }

    // Snow. Three depths, the nearest biggest and fastest, which is the only
    // parallax a camera at a fixed height can offer.
    if (params.flakes > 0.002) {
        let radii = vec3<f32>(0.060, 0.045, 0.040);
        let speeds = vec3<f32>(0.22, 0.16, 0.11);
        let alphas = vec3<f32>(0.55, 0.40, 0.28);
        for (var i = 0; i < 3; i = i + 1) {
            let j = f32(i);
            let travel = fall * (speeds[i] * t) + wind * (speeds[i] / speeds[0]);
            let m = mark_at(table, 1.8, travel, 8.0 + 4.0 * j, j, t);
            let wobble = across * (0.05 * sin(t * (0.4 + 0.5 * m.s1) + m.s2 * TAU));
            let d = length(m.at - wobble);
            let r = radii[i];
            let body = 1.0 - smoothstep(0.4 * r, r, d);
            let a = body * m.env * alive(m, 0.45, params.flakes)
                * alphas[i] * (0.7 + 0.3 * params.flakes);
            acc = over(acc, to_linear(FLAKE), a);
        }
    }

    // Forest. Two depths, each leaf drawn twice — its shadow on the felt
    // first, then the leaf displaced away from it.
    if (params.leaves > 0.002) {
        let speeds = vec2<f32>(0.55, 0.38);
        let scales = vec2<f32>(1.00, 0.80);
        let trims = vec2<f32>(1.00, 0.85);
        for (var i = 0; i < 2; i = i + 1) {
            let j = f32(i);
            let travel = fall * (speeds[i] * t) + wind * (speeds[i] / speeds[0]);
            let m = mark_at(table, 4.0, travel, 9.0, j + 4.0, t);
            let here = alive(m, 0.45, params.leaves);
            // Edge-on twice a turn, drawn as the short axis collapsing.
            let tumble = t * 1.3 * (0.8 + 0.4 * m.s1) + m.s2 * TAU;
            let face = abs(cos(tumble));
            let heading = atan2(fall.y, fall.x)
                + (m.s1 - 0.5) * 0.8
                + 0.30 * sin(t * 0.9 + m.s2 * TAU);
            // The sideslip happens as the leaf turns edge-on, which is what a
            // real leaf does and what a straight fall never looks like.
            let slip = turn(vec2<f32>(0.0, 1.0), heading) * (0.12 * cos(tumble));
            let length_of = 0.18 * scales[i];
            let width_of = max(0.10 * scales[i] * max(0.35, face), 0.035);
            let alpha = 0.75 * (0.40 + 0.60 * face) * trims[i]
                * m.env * here * (0.7 + 0.3 * params.leaves);

            // The shadow the leaf lays on the cloth, and then the leaf a
            // tenth of a unit away from it along the fall. That offset is the
            // card rule (`tabletop::card_shadow`, `table::CAMERA_LEAN`) and it
            // is the single detail that says *in the air over a table* rather
            // than *on the lens*: the eye is already reading card height by
            // it on the same screen.
            let shadow_at = turn(m.at - slip, -heading);
            let sd = length(vec2<f32>(shadow_at.x / (length_of * 1.08), shadow_at.y / (width_of * 1.08)));
            let shade = 1.0 - smoothstep(-0.03, 0.0, (sd - 1.0) * min(length_of, width_of));
            acc = over(acc, vec3<f32>(0.0), shade * alpha * 0.40);

            let leaf_at = turn(m.at - slip + fall * 0.10, -heading);
            let ld = length(vec2<f32>(leaf_at.x / length_of, leaf_at.y / width_of));
            let body = 1.0 - smoothstep(-0.03, 0.0, (ld - 1.0) * min(length_of, width_of));
            let pick = hash2(vec2<f32>(m.s1 * 71.0, m.s2 * 71.0));
            var rgb = LEAF_GOLD;
            if (pick < 0.45) {
                rgb = LEAF_OCHRE;
            } else if (pick < 0.80) {
                rgb = LEAF_RUST;
            }
            // The midrib: a darker line down the long axis, which is what
            // reads as a leaf rather than as a grain of rice.
            let rib = 1.0 - 0.45 * (1.0 - smoothstep(0.02, 0.04, abs(leaf_at.y)));
            acc = over(acc, to_linear(rgb) * rib, body * alpha);
        }
    }

    // Mountain. Rising against the fall, and going out rather than fading:
    // the envelope *is* the cooling, and the colour travels with it.
    if (params.embers > 0.002) {
        let cores = vec2<f32>(0.06, 0.04);
        let halos = vec2<f32>(0.22, 0.16);
        let speeds = vec2<f32>(0.50, 0.32);
        for (var i = 0; i < 2; i = i + 1) {
            let j = f32(i);
            let travel = -fall * (speeds[i] * t) + wind * (speeds[i] / speeds[0]);
            let m = mark_at(table, 2.5, travel, 2.5 + 2.0 * j, j + 8.0, t);
            let wobble = across * (0.06 * sin(t * (1.4 + 1.2 * m.s1) + m.s2 * TAU));
            let d = length(m.at - wobble);
            // A spark has no edge. The core was a plateau with a cliff at its
            // rim — measured live, 187 of red across twenty pixels and then
            // 64 three pixels later — which reads as a counter somebody left
            // on the felt rather than as something burning. It falls off from
            // the middle now and meets the halo without a step in between.
            let core = pow(1.0 - smoothstep(0.0, cores[i] * 2.4, d), 3.0);
            let glow = pow(1.0 - smoothstep(0.0, halos[i], d), 2.0);
            // An ember cools rather than fading symmetrically: it is at its
            // brightest early and spends most of its life going out.
            let burn = smoothstep(0.0, 0.08, m.phase) * (1.0 - smoothstep(0.55, 1.0, m.phase));
            let flicker = 0.78 + 0.22 * sin(t * (7.0 + 5.0 * m.s2) + m.s1 * TAU);
            let heat = burn * flicker * alive(m, 0.35, params.embers)
                * (0.7 + 0.3 * params.embers);
            var rgb = mix(EMBER_BIRTH, EMBER_MID, smoothstep(0.0, 0.45, m.phase));
            rgb = mix(rgb, EMBER_ASH, smoothstep(0.45, 1.0, m.phase));
            let lit = to_linear(rgb);
            // The halo is the ember's own light on the cloth and is not
            // displaced: that is its ground cue, the way the shadow is the
            // leaf's.
            acc = over(acc, lit, glow * heat * 0.22);
            acc = over(acc, lit, core * heat * 0.75);
        }
    }

    if (acc.a <= 0.0005) {
        // Nothing in the air here. A transparent fragment rather than a
        // `discard`, which keeps the branch uniform.
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
    // Un-premultiply: `acc.rgb` is the sum of colour × coverage, so this is
    // the colour the cloth is being pulled towards, and `acc.a` is how far.
    return vec4<f32>(acc.rgb / acc.a, acc.a);
}

/// The island's mist: a sheet that drifts *over* the table.
fn veil_mist(table: vec2<f32>, t: f32, wind: vec2<f32>, across: vec2<f32>, inset: f32) -> f32 {
    if (params.mist <= 0.002) {
        return 0.0;
    }
    let creep = turn(across, 0.349) * (0.10 * t);
    let p = (table - wind - creep) * 0.13;
    let warp = fbm(p * 0.7 + t * 0.02);
    let field = fbm(p + warp * 0.6 + 5.1);
    // Patchy, and steeply so: a field that never reaches zero is a filter
    // over the whole table rather than weather in part of it.
    return params.mist * MIST_PEAK
        * smoothstep(0.42, 0.78, field)
        * smoothstep(0.0, MIST_SHORE, inset);
}

/// The swamp's fog, as a field: warped in place rather than translated.
fn fog_field(table: vec2<f32>, t: f32) -> f32 {
    let p = table * 0.07;
    let wv = vec2<f32>(fbm(p * 0.5 + t * 0.012), fbm(p * 0.5 - t * 0.009 + 8.3));
    return fbm(p + (wv - vec2<f32>(0.5)) * 1.6 + 2.9);
}

/// The swamp's fog: it lies *in* the table.
///
/// Three things say so and none of them is the colour alone. It **darkens**
/// where the mist lifts — the body is display luma 0.144 against the cloth's
/// 0.206 — it churns in place instead of travelling anywhere, because a thing
/// lying in a hollow goes nowhere, and it stops a whole unit further from the
/// rail than the mist does.
fn veil_fog(table: vec2<f32>, t: f32, half: vec2<f32>, inset: f32) -> f32 {
    if (params.fog <= 0.002) {
        return 0.0;
    }
    let pool = smoothstep(0.35, 0.80, fog_field(table, t));
    let radial = table / max(half, vec2<f32>(1e-3));
    let r2 = clamp(dot(radial, radial), 0.0, 1.0);
    return params.fog * FOG_PEAK
        * pool
        * (0.6 + 0.4 * (1.0 - r2))
        * (0.85 + 0.15 * sin(t * 0.27))
        * smoothstep(0.0, FOG_SHORE, inset);
}
