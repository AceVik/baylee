// The shells round a permanent (#298): `shellmat.rs` says what each is and
// why it may stand where it stands.
//
// Its own vertex stage, for the one thing Bevy's does not hand on: the
// camera in the shell's own space, whose `z = 0` is the card's face. With it
// the fragment follows its own ray down to the face, and where that ray
// meets the print the shell is not drawn at all (`clear_over_print`). Every
// colour this file returns is multiplied by that but a dome's, which is
// glass over its whole card by the owner's choice (25.09); a test reads
// every `return` below to make sure it still is.
//
// The steel is darksteel, Magic's own indestructible metal (the owner,
// 25.09): nearly black, darker than the felt it stands on, and read as
// metal by what it mirrors, the room's dim sky over a crisp horizon, by a
// glint and by the silver sheen going round it. The domes are glass in the
// lobby's blue-hour key, their colour in `params.tint`, and a dome lying
// down is a band of plates in that colour, crisp-edged, never the offer's
// soft light.
// Defender's wall is brick at dusk, standing on the felt under every face,
// so the prints in front of it hide it by depth (it writes depth, being
// solid); the mask is there all the same, and on the shadows a standing
// dome and the wall cast on the felt.

#import bevy_pbr::mesh_functions::{get_world_from_local, get_local_from_world, mesh_position_local_to_world, mesh_normal_local_to_world}
#import bevy_pbr::view_transformations::position_world_to_clip
#import bevy_pbr::mesh_view_bindings::{view, globals}
#import "embedded://baylee_client/shaders/card_common.wgsl"::perimeter

struct ShellParams {
    /// A dome's colour, linear; the steel ignores it.
    tint: vec4<f32>,
    /// `shellmat::ShellKind`.
    kind: u32,
    /// The clock every animated term runs on: 1 normally, 0 for
    /// `Preferences::reduce_motion`.
    motion: f32,
    /// Where a ring's band starts and ends past the card's edge.
    inner: f32,
    outer: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> params: ShellParams;

const SHELL_RIM: u32 = 1u;
const SHELL_RING: u32 = 2u;
const SHELL_DOME: u32 = 3u;
const SHELL_DOME_RING: u32 = 4u;
const SHELL_WALL: u32 = 5u;
const SHELL_SHADE: u32 = 6u;

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
/// How far a ring's edges take to fade in.
const SOFT: f32 = 0.012;
/// The share of the rim's slope that is its flat lip, along its crest.
const RIM_LIP: f32 = 0.25;
/// How steeply a dome's glass deepens from its crown to its foot.
const DOME_RAMP: f32 = 3.0;
/// Where the moon's arc lies on a dome, from its crown (0) to its foot (1),
/// how far it reaches either side of that, and how bright it is.
const ARC_AT: f32 = 0.9;
const ARC_HALF: f32 = 0.06;
const ARC_GLOW: f32 = 0.85;
/// How high over the felt a shadow on it lies, in the world:
/// `shellmat::SHADE_RUNG`, the felt being at `table::TABLE_Y`, 0; and how
/// far over that it is gone, which is under every card's face.
const SHADE_FLOOR: f32 = 0.0168;
const SHADE_GONE: f32 = 0.04;
/// How many plates a lying dome's band has along each short side of the
/// card, and along each long one.
const PLATES_SHORT: f32 = 6.0;
const PLATES_LONG: f32 = 8.0;
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
/// A shadow on the felt, the blue hour's darkest.
const SHADE: vec3<f32> = vec3<f32>(0.0, 0.002, 0.008);

struct ShellVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct ShellOut {
    @builtin(position) clip: vec4<f32>,
    /// The point and the camera in the shell's own space.
    @location(0) local_pos: vec3<f32>,
    @location(1) local_cam: vec3<f32>,
    @location(2) world_position: vec3<f32>,
    @location(3) world_normal: vec3<f32>,
    /// The card's own UV at this point, running on past its edges; on a
    /// dome, `x` is how far out from its crown to its foot it is, seen from
    /// above (`shellmat::dome_mesh`).
    @location(4) uv: vec2<f32>,
    /// [`KEY`] in the shell's own space.
    @location(5) local_key: vec3<f32>,
    /// The mesh's normal, in the shell's own space.
    @location(6) local_normal: vec3<f32>,
};

@vertex
fn vertex(v: ShellVertex) -> ShellOut {
    var out: ShellOut;
    let world_from_local = get_world_from_local(v.instance_index);
    let world = mesh_position_local_to_world(world_from_local, vec4<f32>(v.position, 1.0));
    out.clip = position_world_to_clip(world.xyz);
    out.world_position = world.xyz;
    out.world_normal = mesh_normal_local_to_world(v.normal, v.instance_index);
    out.local_pos = v.position;
    // One camera for the whole shell, so interpolating it is exact.
    out.local_cam = (get_local_from_world(v.instance_index) * vec4<f32>(view.world_position, 1.0)).xyz;
    out.local_key = normalize((get_local_from_world(v.instance_index) * vec4<f32>(KEY, 0.0)).xyz);
    out.local_normal = v.normal;
    out.uv = v.uv;
    return out;
}

/// Signed distance to the card's rounded rectangle, in its own space.
fn card_sdf(p: vec2<f32>) -> f32 {
    let half = vec2<f32>(CARD_HALF_W, CARD_HALF_H);
    let q = abs(p) - (half - vec2<f32>(CARD_ROUND));
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - CARD_ROUND;
}

/// Zero where the ray from `cam` through `p` meets the card's face on the
/// print; one once it meets it `MASK_FEATHER` off it. `shellmat::clear_over_print`.
fn clear_over_print(cam: vec3<f32>, p: vec3<f32>) -> f32 {
    let t = cam.z / max(cam.z - p.z, 1e-4);
    let hit = cam.xy + (p.xy - cam.xy) * t;
    return smoothstep(0.0, MASK_FEATHER, card_sdf(hit));
}

/// Which way is out from the card at `p`, in its own plane.
fn card_out(p: vec2<f32>) -> vec2<f32> {
    let q = abs(p) - (vec2<f32>(CARD_HALF_W, CARD_HALF_H) - vec2<f32>(CARD_ROUND));
    return sign(p) * normalize(max(q, vec2<f32>(1e-5)));
}

/// Darksteel, all in the shell's own space, where `z` is up off the face:
/// the metal, nearly black, and what it mirrors of the room round the table,
/// the dim blue hour over the horizon and the dark felt under it, with the
/// crisp line between them that tells metal from paint; a glint of [`KEY`];
/// and a silver band of light going round the card (`band`, at its mean
/// when still). `n` is the surface's shape, not its mesh's: a rounded bevel
/// the mesh is too coarse to carry.
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

/// Where along a lying dome's band of plates `uv` is: how far into its
/// plate (0..1), from the plate counts per side.
fn plate_at(uv: vec2<f32>) -> f32 {
    let round = perimeter(uv) * 4.0;
    let side = floor(round);
    let count = select(PLATES_SHORT, PLATES_LONG, side == 1.0 || side == 3.0);
    return fract(fract(round) * count);
}

/// How much of a lying dome's band is here: plates with gaps between, and
/// crisp edges where the offer's light is soft.
fn plate_alpha(uv: vec2<f32>, d: f32, aa: f32) -> f32 {
    let f = plate_at(uv);
    let w = max(aa, 1e-4);
    let across = smoothstep(params.inner - w, params.inner + w, d)
        * (1.0 - smoothstep(params.outer - w, params.outer + w, d));
    let along = smoothstep(0.06, 0.12, f) * (1.0 - smoothstep(0.88, 0.94, f));
    return across * along;
}

/// A lying dome's plate: its colour, a darker fill and a bright line along
/// both edges of the band (a double line), breathing as the dome does.
fn plates(uv: vec2<f32>, d: f32, breath: f32) -> vec3<f32> {
    let width = params.outer - params.inner;
    let t = clamp((d - params.inner) / max(width, 1e-4), 0.0, 1.0);
    let lines = max(1.0 - smoothstep(0.0, 0.3, t), smoothstep(0.7, 1.0, t));
    return params.tint.rgb * (0.45 + 0.9 * lines) * breath;
}

/// Defender's wall in running bond: `along` is the distance along its arc,
/// `h` the height off the felt, `aa` how far either moves across a pixel.
/// Lit by [`KEY`], the blue hour where it turns away; its courses in pale
/// mortar, which a duel's camera reads on the face leaning back towards it.
/// A top shows only the joints across it: pale stone coping on the merlons,
/// brick in their shade between them.
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

@fragment
fn fragment(in: ShellOut) -> @location(0) vec4<f32> {
    // Nothing of the rim, the rings or the wall over its own print.
    let clear = clear_over_print(in.local_cam, in.local_pos);
    // Taken before any branch, where every pixel of the quad still runs.
    let aa = max(fwidth(in.uv.x), fwidth(in.local_pos.z));
    let aa_d = fwidth(card_sdf(in.local_pos.xy));

    let m = params.motion;
    let n = normalize(in.world_normal);
    let v = normalize(view.world_position - in.world_position);
    // One band of light round the card, a lap every seven seconds; with
    // motion off, its mean everywhere, which is 1/7 of its peak.
    let lap = 1.0 - 2.0 * abs(fract(perimeter(in.uv) - 0.15 * globals.time * m) - 0.5);
    let band = mix(1.0 / 7.0, pow(lap, 6.0), m);
    // How far down the rim's slope this is: 0 at its top edge, 1 at its foot.
    let down = clamp((RIM_RISE - in.local_pos.z) / (RIM_RISE + RIM_DROP), 0.0, 1.0);

    // How far past the card's edge this point is.
    let d = card_sdf(in.local_pos.xy);
    // A dome's light breathes between 1 and 1.1 once every six seconds; with
    // motion off, its mean.
    let breath = 1.05 + 0.05 * m * sin(globals.time * 1.047);
    if params.kind == SHELL_DOME_RING {
        return vec4<f32>(plates(in.uv, d, breath), plate_alpha(in.uv, d, aa_d) * clear);
    }
    if params.kind == SHELL_SHADE {
        // A shadow on the felt: darkest where it is cast (`uv.y`), soft to
        // nothing by its far edge, and gone as its card lifts it off the
        // felt.
        let fade = 1.0 - smoothstep(0.0, 1.0, in.uv.x);
        let grounded = 1.0 - smoothstep(SHADE_FLOOR + 0.004, SHADE_FLOOR + SHADE_GONE, in.world_position.y);
        return vec4<f32>(SHADE, in.uv.y * fade * grounded * clear);
    }
    let out = card_out(in.local_pos.xy);
    let lv = normalize(in.local_cam - in.local_pos);
    if params.kind == SHELL_RING {
        // A rod of darksteel lying on the felt, round across: its band
        // tipped inward at its inner edge, outward at its outer, and soft at
        // both, so it lies on the felt rather than cut into it.
        let body = smoothstep(params.inner, params.inner + SOFT, d)
            * (1.0 - smoothstep(params.outer - SOFT, params.outer, d));
        let across = clamp(2.0 * (d - params.inner) / max(params.outer - params.inner, 1e-4) - 1.0, -1.0, 1.0);
        let colour = steel(tipped(out, 1.3 * asin(across) / 1.5708), lv, in.local_key, band);
        return vec4<f32>(colour, 0.9 * body * clear);
    }
    if params.kind == SHELL_DOME {
        // Glass, seen from nearly over it as a duel sees it: the more of it
        // a ray crosses, the deeper its colour, which from above is how far
        // out from its crown towards its foot the point lies (`uv.x`). So
        // nearly clear at the crown, deepening smoothly to its foot with no
        // edge on the way, a little brighter on the side turned to the
        // light. And the moon mirrored in it: one arc on its shoulder, round
        // the side that faces halfway between the light and the camera, as
        // far round as it faces them.
        let deep = pow(in.uv.x, DOME_RAMP);
        let toward = dot(n.xz, normalize(KEY.xz));
        let lit = 0.55 + 0.45 * toward;
        let body = (0.03 + 0.55 * deep * lit) * breath;
        let half = normalize(lv + in.local_key);
        let slope = in.local_normal.xy / max(length(in.local_normal.xy), 1e-4);
        let round = smoothstep(0.35, 0.9, dot(slope, half.xy / max(length(half.xy), 1e-4)));
        let across = 1.0 - smoothstep(0.0, ARC_HALF, abs(in.uv.x - ARC_AT));
        let arc = ARC_GLOW * round * across * across;
        let glass = min(body + arc, 0.92);
        let pale = mix(params.tint.rgb, MOON, 0.3 * max(toward, 0.0) * deep);
        let colour = mix(pale, MOON, arc / max(body + arc, 1e-4));
        return vec4<f32>(colour, glass);
    }
    if params.kind == SHELL_WALL {
        return vec4<f32>(brick(in.uv.x, in.local_pos.z + RIM_DROP, n, aa), clear);
    }
    // The rim: a flat lip along its crest, facing up, mirroring the sky and
    // the brightest of it; then rounded to its foot, facing out, as a
    // quarter round is, so it mirrors the felt; darker towards its foot,
    // where it meets the felt.
    let bevel = max(down - RIM_LIP, 0.0) / (1.0 - RIM_LIP);
    let colour = steel(tipped(out, acos(1.0 - bevel)), lv, in.local_key, band);
    return vec4<f32>(colour * (1.0 - 0.3 * down), clear);
}
