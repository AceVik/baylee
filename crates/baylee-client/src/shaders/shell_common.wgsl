// What the shells round a permanent share between the table (`shell.wgsl`)
// and the preview (`shell_ui.wgsl`): the card they stand round, and how
// darksteel, glass and brick are lit. `shellmat.rs` says what each shell is
// and why it may stand where it stands; the two shaders say where it is seen
// from, the table's through its real camera and the preview's from straight
// over the card.
//
// Everything here is in a shell's own space, the card's: its middle at the
// origin, `y` up the card, `z` up off its face, in card widths. Where a term
// needs a direction in the world it says so.

/// Which shell a material draws. `shellmat::ShellKind`.
const SHELL_RIM: u32 = 1u;
const SHELL_RING: u32 = 2u;
const SHELL_DOME: u32 = 3u;
const SHELL_DOME_RING: u32 = 4u;
const SHELL_WALL: u32 = 5u;
const SHELL_SHADE: u32 = 6u;
const SHELL_WAVE: u32 = 7u;

/// The card, in its own space: half its width and height, and its corner.
/// `shellmat` holds these to `CARD_WIDTH`, `CARD_HEIGHT` and `CARD_CORNER`.
const CARD_HALF_W: f32 = 0.5;
const CARD_HALF_H: f32 = 0.6985;
const CARD_ROUND: f32 = 0.0476;
/// How far off the print the mask takes to open: outward only.
const MASK_FEATHER: f32 = 0.012;
/// The rim's top edge over the face, and its foot under it.
const RIM_RISE: f32 = 0.01;
const RIM_DROP: f32 = 0.083;
/// The share of the rim's slope that is its flat lip, along its crest.
const RIM_LIP: f32 = 0.25;
/// How steeply a dome's glass deepens from its crown to its foot.
const DOME_RAMP: f32 = 3.0;
/// Where the moon's arc lies on a dome, from its crown (0) to its foot (1),
/// how far it reaches either side of that, and how bright it is.
const ARC_AT: f32 = 0.9;
const ARC_HALF: f32 = 0.06;
const ARC_GLOW: f32 = 0.85;
/// One course of the wall's brick, one brick along it, the mortar between
/// them, and the wall's height over the felt, its merlons' tops.
const WALL_COURSE: f32 = 0.028;
const WALL_BRICK: f32 = 0.07;
const WALL_MORTAR: f32 = 0.01;
const WALL_HEIGHT: f32 = 0.08;

/// Darksteel, linear: the metal, and the light it gives back, a cool
/// silver.
const STEEL: vec3<f32> = vec3<f32>(0.006, 0.0065, 0.0078);
const SHEEN: vec3<f32> = vec3<f32>(0.58, 0.60, 0.64);
/// What the steel mirrors of the felt under the horizon: dim, not black.
const FELT_MIRROR: f32 = 0.03;
/// The light every shell catches, from over the player's shoulder, in the
/// world; and its colour on glass, the moon's.
const KEY: vec3<f32> = vec3<f32>(-0.293, 0.838, 0.461);
const MOON: vec3<f32> = vec3<f32>(0.80, 0.86, 1.0);
/// Brick, pale mortar, the stone coping its merlons, linear, and the blue
/// hour in the wall's shade.
const BRICK: vec3<f32> = vec3<f32>(0.30, 0.075, 0.04);
const MORTAR: vec3<f32> = vec3<f32>(0.30, 0.30, 0.32);
const COPING: vec3<f32> = vec3<f32>(0.42, 0.38, 0.33);
const DUSK: vec3<f32> = vec3<f32>(0.006, 0.01, 0.025);

/// Signed distance to the card's rounded rectangle, in its own space.
fn card_sdf(p: vec2<f32>) -> f32 {
    let half = vec2<f32>(CARD_HALF_W, CARD_HALF_H);
    let q = abs(p) - (half - vec2<f32>(CARD_ROUND));
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - CARD_ROUND;
}

/// Which way is out from the card at `p`, in its own plane.
fn card_out(p: vec2<f32>) -> vec2<f32> {
    let q = abs(p) - (vec2<f32>(CARD_HALF_W, CARD_HALF_H) - vec2<f32>(CARD_ROUND));
    return sign(p) * normalize(max(q, vec2<f32>(1e-5)));
}

/// Darksteel: the metal, nearly black, and what it mirrors of the room round
/// the table, the dim blue hour over the horizon and the dark felt under it,
/// with the crisp line between them that tells metal from paint; a glint of
/// [`KEY`]; and a silver band of light going round the card (`band`, at its
/// mean when still). `n` is the surface's shape, not its mesh's: a rounded
/// bevel the mesh is too coarse to carry. `v` is towards the eye and `key`
/// is [`KEY`], both in the shell's own space.
fn steel(n: vec3<f32>, v: vec3<f32>, key: vec3<f32>, band: f32) -> vec3<f32> {
    let r = reflect(-v, n);
    let sky = mix(FELT_MIRROR, 0.1 + 0.25 * max(r.z, 0.0), smoothstep(-0.02, 0.02, r.z));
    let glint = pow(max(dot(r, key), 0.0), 12.0);
    return STEEL + SHEEN * (sky + 0.6 * glint + 0.45 * band);
}

/// A surface tipped `tilt` (radians) from facing straight up towards `out`.
fn tipped(out: vec2<f32>, tilt: f32) -> vec3<f32> {
    return vec3<f32>(out * sin(tilt), cos(tilt));
}

/// The rim `down` its slope (0 at its top edge, 1 at its foot), facing
/// `out` from the card: a flat lip along its crest, facing up, mirroring the
/// sky and the brightest of it; then rounded to its foot, facing out, as a
/// quarter round is, so it mirrors the felt; darker towards its foot, where
/// it meets the felt.
fn rim_steel(out: vec2<f32>, down: f32, v: vec3<f32>, key: vec3<f32>, band: f32) -> vec3<f32> {
    let bevel = max(down - RIM_LIP, 0.0) / (1.0 - RIM_LIP);
    return steel(tipped(out, acos(1.0 - bevel)), v, key, band) * (1.0 - 0.3 * down);
}

/// A dome's glass and how much of it there is: its colour and its alpha.
///
/// Seen from nearly over it as a duel sees it: the more of it a ray crosses,
/// the deeper its colour, which from above is how far out from its crown
/// towards its foot the point lies (`reach`, 0 at the crown, 1 at the foot).
/// So nearly clear at the crown, deepening smoothly to its foot with no edge
/// on the way, a little brighter on the side turned to the light. And the
/// moon mirrored in it: one arc on its shoulder, round the side that faces
/// halfway between the light and the eye, as far round as it faces them.
/// `n` is its surface's normal, `v` towards the eye and `key` [`KEY`], all
/// in the shell's own space; `breath` its light's slow swell.
fn dome_glass(
    reach: f32,
    n: vec3<f32>,
    v: vec3<f32>,
    key: vec3<f32>,
    tint: vec3<f32>,
    breath: f32,
) -> vec4<f32> {
    let deep = pow(reach, DOME_RAMP);
    let slope = n.xy / max(length(n.xy), 1e-4);
    let toward = dot(n.xy, key.xy / max(length(key.xy), 1e-4));
    let lit = 0.55 + 0.45 * toward;
    let body = (0.03 + 0.55 * deep * lit) * breath;
    let half = normalize(v + key);
    let round = smoothstep(0.35, 0.9, dot(slope, half.xy / max(length(half.xy), 1e-4)));
    let across = 1.0 - smoothstep(0.0, ARC_HALF, abs(reach - ARC_AT));
    let arc = ARC_GLOW * round * across * across;
    let glass = min(body + arc, 0.92);
    let pale = mix(tint, MOON, 0.3 * max(toward, 0.0) * deep);
    return vec4<f32>(mix(pale, MOON, arc / max(body + arc, 1e-4)), glass);
}

/// Defender's wall in running bond: `along` is the distance along its arc,
/// `h` the height off the felt, `aa` how far either moves across a pixel.
/// `n` is the surface's normal in the world, `y` up. Lit by [`KEY`], the
/// blue hour where it turns away; its courses in pale mortar, which a duel's
/// camera reads on the face leaning back towards it. A top shows only the
/// joints across it: pale stone coping on the merlons, brick in their shade
/// between them.
fn brick(along: f32, h: f32, n: vec3<f32>, aa: f32) -> vec3<f32> {
    let row = floor(h / WALL_COURSE);
    let x = along / WALL_BRICK + 0.5 * (row % 2.0);
    let cell = floor(x);
    let fy = fract(h / WALL_COURSE);
    var joint = min(fract(x), 1.0 - fract(x)) * WALL_BRICK;
    let top = n.y > 0.95;
    if !top {
        joint = min(joint, min(fy, 1.0 - fy) * WALL_COURSE);
    }
    // Where a pixel is nearly as wide as a course, the joints blur into the
    // share of the face they take, about a third, rather than into grey.
    let sharp = 1.0 - smoothstep(0.5 * WALL_MORTAR - aa, 0.5 * WALL_MORTAR + aa, joint);
    let mortar = mix(0.3, sharp, 1.0 - smoothstep(0.35 * WALL_COURSE, WALL_COURSE, aa));
    // Every brick a shade of its own, from where it lies.
    let shade = 0.85 + 0.3 * fract(sin(dot(vec2<f32>(cell, row), vec2<f32>(12.9898, 78.233))) * 43758.5453);
    let coping = top && h > WALL_HEIGHT - 0.002;
    let stone = select(BRICK * shade, COPING * (0.9 + 0.2 * shade), coping);
    let crenel = select(1.0, 0.55, top && !coping);
    let up = max(n.y, 0.0);
    let lit = 0.3 + 0.8 * max(dot(n, KEY), 0.0);
    return mix(stone, MORTAR, mortar) * lit * crenel + DUSK * (1.0 - up);
}
