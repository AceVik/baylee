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
// Summoning sickness's wave is a sheet over the face that this vertex stage
// raises a crest in, running out from the card's middle and resting;
// moonlight on its slopes, and over its own print by the owner's choice
// (25.09), as the dome's glass is.
//
// The card, the steel, the glass and the brick are `shell_common.wgsl`'s,
// which the preview's shells (`shell_ui.wgsl`) draw with too: this file
// says only where each is seen from, through the real camera.

#import bevy_pbr::mesh_functions::{get_world_from_local, get_local_from_world, mesh_position_local_to_world, mesh_normal_local_to_world}
#import bevy_pbr::view_transformations::position_world_to_clip
#import bevy_pbr::mesh_view_bindings::{view, globals}
#import "embedded://baylee_client/shaders/card_common.wgsl"::perimeter
#import "embedded://baylee_client/shaders/shell_common.wgsl"::{SHELL_RING, SHELL_DOME, SHELL_DOME_RING, SHELL_WALL, SHELL_SHADE, SHELL_WAVE, CARD_HALF_W, MASK_FEATHER, RIM_RISE, RIM_DROP, KEY, WAVE_LIFT, WAVE_CREST, WAVE_PERIOD, MOONLIGHT, card_sdf, card_out, steel, tipped, rim_steel, dome_glass, brick, crest_at, crest_lean, wave_along, wave_glow}

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

/// How far a ring's edges take to fade in.
const SOFT: f32 = 0.012;
/// How high over the felt a shadow on it lies, in the world:
/// `shellmat::SHADE_RUNG`, the felt being at `table::TABLE_Y`, 0; and how
/// far over that it is gone, which is under every card's face.
const SHADE_FLOOR: f32 = 0.0168;
const SHADE_GONE: f32 = 0.04;
/// How many plates a lying dome's band has along each short side of the
/// card, and along each long one.
const PLATES_SHORT: f32 = 6.0;
const PLATES_LONG: f32 = 8.0;

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
    /// On a wave, how far out its front is: 0 at the card's middle, 1 at
    /// its sheet's edge, and past 1 while it rests.
    @location(7) along: f32,
};

@vertex
fn vertex(v: ShellVertex) -> ShellOut {
    var out: ShellOut;
    let world_from_local = get_world_from_local(v.instance_index);
    var local = v.position;
    var normal = v.normal;
    out.along = 0.0;
    if params.kind == SHELL_WAVE {
        // Each card's own moment, from where it lies on the table, so a
        // board of new creatures does not pulse as one; with motion off,
        // one wave held still most of the way out.
        let phase = fract(dot(world_from_local[3].xz, vec2<f32>(0.37, 0.61)));
        out.along = wave_along(globals.time * params.motion / WAVE_PERIOD + phase, params.motion);
        local = vec3<f32>(v.position.xy, WAVE_LIFT + WAVE_CREST * crest_at(v.position.xy, out.along));
        normal = normalize(vec3<f32>(-crest_lean(v.position.xy, out.along), 1.0));
    }
    let world = mesh_position_local_to_world(world_from_local, vec4<f32>(local, 1.0));
    out.clip = position_world_to_clip(world.xyz);
    out.world_position = world.xyz;
    out.world_normal = mesh_normal_local_to_world(normal, v.instance_index);
    out.local_pos = local;
    // One camera for the whole shell, so interpolating it is exact.
    out.local_cam = (get_local_from_world(v.instance_index) * vec4<f32>(view.world_position, 1.0)).xyz;
    out.local_key = normalize((get_local_from_world(v.instance_index) * vec4<f32>(KEY, 0.0)).xyz);
    out.local_normal = v.normal;
    out.uv = v.uv;
    return out;
}

/// Zero where the ray from `cam` through `p` meets the card's face on the
/// print; one once it meets it `MASK_FEATHER` off it. `shellmat::clear_over_print`.
fn clear_over_print(cam: vec3<f32>, p: vec3<f32>) -> f32 {
    let t = cam.z / max(cam.z - p.z, 1e-4);
    let hit = cam.xy + (p.xy - cam.xy) * t;
    return smoothstep(0.0, MASK_FEATHER, card_sdf(hit));
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
    if params.kind == SHELL_WAVE {
        // Shaded here rather than from the mesh, which is coarser than the
        // crest.
        let glow = wave_glow(in.local_pos.xy, in.along, lv, in.local_key);
        return vec4<f32>(MOONLIGHT, glow);
    }
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
        // Glass, seen from nearly over it as a duel sees it (`dome_glass`):
        // how far out from its crown towards its foot the point lies, seen
        // from above, is the mesh's `uv.x`, and its slope the mesh's normal.
        let lit = dome_glass(in.uv.x, normalize(in.local_normal), lv, in.local_key, params.tint.rgb, breath);
        let colour = lit.rgb;
        let glass = lit.a;
        return vec4<f32>(colour, glass);
    }
    if params.kind == SHELL_WALL {
        return vec4<f32>(brick(in.uv.x, in.local_pos.z + RIM_DROP, n, aa), clear);
    }
    // The rim, down its slope from its top edge to its foot (`rim_steel`).
    return vec4<f32>(rim_steel(out, down, lv, in.local_key, band), clear);
}
