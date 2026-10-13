// The shells round a permanent (#298): `shellmat.rs` says what each is and
// why it may stand where it stands.
//
// Its own vertex stage, for the one thing Bevy's does not hand on: the
// camera in the shell's own space, whose `z = 0` is the card's face. With it
// the fragment follows its own ray down to the face, and where that ray
// meets the print the shell is not drawn at all (`clear_over_print`). Every
// colour this file returns is multiplied by that but the wave's, which
// passes over its own print by the owner's choice (25.09); a test reads
// every `return` below to make sure it still is.
//
// A keyword's border (the owner, 13.10.2026) is a slim band standing round
// the card from a hair over its face down to the felt, or lying on the felt
// as a ring where it has no room; its layers and their patterns are
// `shell_common.wgsl`'s `ward_border`, which the preview draws with too.
// Summoning sickness's wave is a sheet over the face that this vertex stage
// raises a crest in, running out from the card's middle and resting.

#import bevy_pbr::mesh_functions::{get_world_from_local, get_local_from_world, mesh_position_local_to_world}
#import bevy_pbr::view_transformations::position_world_to_clip
#import bevy_pbr::mesh_view_bindings::{view, globals}
#import "embedded://baylee_client/shaders/card_common.wgsl"::perimeter
#import "embedded://baylee_client/shaders/shell_common.wgsl"::{SHELL_RING, SHELL_WAVE, MASK_FEATHER, RIM_RISE, RIM_DROP, KEY, WAVE_LIFT, WAVE_CREST, WAVE_PERIOD, MOONLIGHT, card_sdf, crest_at, crest_lean, wave_along, wave_glow, ward_border}

struct ShellParams {
    /// `shellmat::ShellKind`.
    kind: u32,
    /// The clock every animated term runs on: 1 normally, 0 for
    /// `Preferences::reduce_motion`.
    motion: f32,
    /// Where a ring's band starts and ends past the card's edge.
    inner: f32,
    outer: f32,
    /// The keywords a border says (`shellmat::Ward`), and how many layers
    /// they make.
    ward: u32,
    layers: u32,
    /// Nothing: the second row's rest.
    spare: vec2<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> params: ShellParams;

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
    /// The card's own UV at this point, running on past its edges.
    @location(2) uv: vec2<f32>,
    /// [`KEY`] in the shell's own space.
    @location(3) local_key: vec3<f32>,
    /// On a wave, how far out its front is: 0 at the card's middle, 1 at
    /// its sheet's edge, and past 1 while it rests.
    @location(4) along: f32,
};

@vertex
fn vertex(v: ShellVertex) -> ShellOut {
    var out: ShellOut;
    let world_from_local = get_world_from_local(v.instance_index);
    var local = v.position;
    out.along = 0.0;
    if params.kind == SHELL_WAVE {
        // Each card's own moment, from where it lies on the table, so a
        // board of new creatures does not pulse as one; with motion off,
        // one wave held still most of the way out.
        let phase = fract(dot(world_from_local[3].xz, vec2<f32>(0.37, 0.61)));
        out.along = wave_along(globals.time * params.motion / WAVE_PERIOD + phase, params.motion);
        local = vec3<f32>(v.position.xy, WAVE_LIFT + WAVE_CREST * crest_at(v.position.xy, out.along));
    }
    let world = mesh_position_local_to_world(world_from_local, vec4<f32>(local, 1.0));
    out.clip = position_world_to_clip(world.xyz);
    out.local_pos = local;
    // One camera for the whole shell, so interpolating it is exact.
    out.local_cam = (get_local_from_world(v.instance_index) * vec4<f32>(view.world_position, 1.0)).xyz;
    out.local_key = normalize((get_local_from_world(v.instance_index) * vec4<f32>(KEY, 0.0)).xyz);
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

@fragment
fn fragment(in: ShellOut) -> @location(0) vec4<f32> {
    // Nothing of a border or a ring over its own print.
    let clear = clear_over_print(in.local_cam, in.local_pos);
    // How far across the band this point is: a standing border down its
    // slope from its top edge to its foot, a ring out from its inner edge.
    // Both taken, with their derivatives, before any branch, where every
    // pixel of the quad still runs.
    let down = (RIM_RISE - in.local_pos.z) / (RIM_RISE + RIM_DROP);
    let ring_out = (card_sdf(in.local_pos.xy) - params.inner) / max(params.outer - params.inner, 1e-4);
    let across = select(down, ring_out, params.kind == SHELL_RING);
    let s = perimeter(in.uv);
    let aa = fwidth(across);
    // The perimeter jumps from 1 back to 0 at the card's top-left corner.
    let aa_s = min(fwidth(s), 0.01);
    let t = globals.time * params.motion;

    if params.kind == SHELL_WAVE {
        // Shaded here rather than from the mesh, which is coarser than the
        // crest.
        let lv = normalize(in.local_cam - in.local_pos);
        let glow = wave_glow(in.local_pos.xy, in.along, lv, in.local_key);
        return vec4<f32>(MOONLIGHT, glow);
    }
    let lit = ward_border(params.ward, params.layers, across, s, t, aa, aa_s);
    return vec4<f32>(lit.rgb, lit.a * clear);
}
