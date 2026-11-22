// The approved gallery finish is applied to the original, straight-alpha paintings.
// Bevy samples sRGB textures in linear light; the gallery's colour masks use sRGB.
import bevy_ui_render::ui_vertex_output::UiVertexOutput;
import bevy_render::globals::Globals;
import baylee_client::rank_art::finish::{Finish, finished_linear, srgb};
import baylee_client::rank_art::portrait::{frame, unranked};
struct Params {
    kind: vec4<u32>,
    timing: vec4<f32>,
    metal: vec4<f32>,
    gem: vec4<f32>,
}
@group(0) @binding(1) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> params: Params;
@group(1) @binding(1) var painting: texture_2d<f32>;
@group(1) @binding(2) var paint_sampler: sampler;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let kind = params.kind;
    let moving = params.timing.x > 0.0 && params.timing.y > 0.0;
    let phase = select(0.34, fract(globals.time / max(params.timing.x, 1.0)), moving);
    if kind.w >= 2u {
        return frame(in.uv, kind, params.metal.rgb, params.gem.rgb, phase);
    }
    if kind.x == 11u {
        return unranked(in.uv, kind.w == 1u, params.metal.rgb);
    }
    let src = textureSample(painting, paint_sampler, in.uv);
    if src.a == 0.0 { return src; }
    let fx = Finish(phase, kind.x, kind.w, kind.y, kind.z, 1u, 0u, 0u);
    return finished_linear(fx, vec4<f32>(srgb(src.rgb), src.a), src, in.uv);
}
