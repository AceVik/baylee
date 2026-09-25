// The shells round a permanent (#298): `shellmat.rs` says what each is and
// why it may stand where it stands.
//
// Its own vertex stage, for the one thing Bevy's does not hand on: the
// camera in the shell's own space, whose `z = 0` is the card's face. With it
// the fragment follows its own ray down to the face, and where that ray
// meets the print the shell is not drawn at all (`clear_over_print`). Every
// colour this file returns is multiplied by that, and a test reads every
// `return` below to make sure it still is.
//
// The steel is darksteel, Magic's own indestructible metal (the owner,
// 25.09): nearly black, darker than the felt it stands on, and read by the
// pale light along its crest and the silver band going round it. The domes
// are light in the lobby's blue-hour key, their colour in `params.tint`.

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
    /// Where a ring's band, or a dome's foot line, starts and ends past the
    /// card's edge.
    inner: f32,
    outer: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> params: ShellParams;

const SHELL_RIM: u32 = 1u;
const SHELL_RING: u32 = 2u;
const SHELL_DOME: u32 = 3u;
const SHELL_DOME_RING: u32 = 4u;

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
/// How far a ring's edges and a dome's foot line take to fade in.
const SOFT: f32 = 0.012;

/// Darksteel, linear: the metal, and the light it gives back, a cool
/// silver.
const STEEL: vec3<f32> = vec3<f32>(0.006, 0.0065, 0.0078);
const SHEEN: vec3<f32> = vec3<f32>(0.58, 0.60, 0.64);

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
    /// The card's own UV at this point, running on past its edges.
    @location(4) uv: vec2<f32>,
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

/// Darksteel under a light from over the player's shoulder: the metal, a
/// pale edge where the surface turns away, a glint, a lip of light along its
/// crest (`lip`, 1 at the top edge), and a silver band of light going round
/// the card (`band`, at its mean when still).
fn steel(n: vec3<f32>, v: vec3<f32>, band: f32, lip: f32) -> vec3<f32> {
    let key = normalize(vec3<f32>(-0.35, 1.0, 0.55));
    let lambert = max(dot(n, key), 0.0);
    let glint = pow(max(dot(reflect(-key, n), v), 0.0), 28.0);
    let edge = pow(1.0 - abs(dot(n, v)), 3.0);
    return STEEL * (0.7 + 0.9 * lambert)
        + SHEEN * (0.04 * edge + 0.5 * glint + 0.25 * lip + 0.08 * band);
}

@fragment
fn fragment(in: ShellOut) -> @location(0) vec4<f32> {
    // Nothing of a shell over its own print.
    let clear = clear_over_print(in.local_cam, in.local_pos);

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
    if params.kind == SHELL_RING || params.kind == SHELL_DOME_RING {
        // Soft at both edges, so it lies on the felt rather than cut into it.
        let body = smoothstep(params.inner, params.inner + SOFT, d)
            * (1.0 - smoothstep(params.outer - SOFT, params.outer, d));
        if params.kind == SHELL_DOME_RING {
            return vec4<f32>(params.tint.rgb, 0.8 * breath * body * clear);
        }
        let colour = steel(n, v, band, 0.0);
        return vec4<f32>(colour, 0.9 * body * clear);
    }
    if params.kind == SHELL_DOME {
        // Light on a surface of glass: faint where it faces the camera,
        // bright where it turns away (from the view alone; there are no
        // lights), a base of it everywhere, and a line along its foot.
        let fresnel = pow(1.0 - abs(dot(n, v)), 2.0);
        let foot = smoothstep(params.inner, params.outer, d)
            * (1.0 - smoothstep(params.outer - 0.3 * SOFT, params.outer, d));
        let glow = (0.05 + 0.55 * fresnel + 0.35 * foot) * breath;
        return vec4<f32>(params.tint.rgb, min(glow, 1.0) * clear);
    }
    // The rim: a lip of light along its crest, darker towards its foot,
    // where it meets the felt.
    let colour = steel(n, v, band, 1.0 - smoothstep(0.0, 0.15, down));
    return vec4<f32>(colour * (1.0 - 0.45 * down), clear);
}
