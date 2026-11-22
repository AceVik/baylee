// The approved gallery finish is applied to the original, straight-alpha paintings.
// Bevy samples sRGB textures in linear light; the gallery's colour masks use sRGB.
import bevy_ui_render::ui_vertex_output::UiVertexOutput;
import bevy_render::globals::Globals;
import baylee_client::rank_art::finish::{Finish, finished_linear, srgb};
import baylee_client::rank_art::portrait::{neutral_frame, engraving, unranked};
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

fn weighted_texel(uv: vec2<f32>) -> vec4<f32> {
    let sample = textureSample(painting, paint_sampler, uv);
    return vec4<f32>(sample.rgb * sample.a, sample.a);
}

// A narrow silver wire can be considerably thinner than one list pixel.
// Integrate the pixel footprint in premultiplied light to preserve its edge
// against every portrait colour, including while its parent is scrolling.
fn mounted_paint(uv: vec2<f32>) -> vec4<f32> {
    let dx = dpdx(uv);
    let dy = dpdy(uv);
    let extent = vec2<f32>(textureDimensions(painting));
    if max(length(dx * extent),length(dy * extent)) <= 1.5 {
        return textureSample(painting,paint_sampler,uv);
    }
    let a = dx * 0.375 + dy * 0.125;
    let b = dy * 0.375 - dx * 0.125;
    let sum = weighted_texel(uv-a) + weighted_texel(uv+a)
            + weighted_texel(uv-b) + weighted_texel(uv+b);
    return vec4<f32>(sum.rgb / max(sum.a, 0.00001), sum.a * 0.25);
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let kind = params.kind;
    let moving = params.timing.x > 0.0 && params.timing.y > 0.0;
    // Bevy's global clock wraps hourly. An integral number of cycles per hour
    // also keeps the seven-second Diamond glint continuous across the wrap.
    let cycles = round(3600.0 / max(params.timing.x, 1.0));
    let phase = select(0.34, fract(globals.time * (cycles / 3600.0)), moving);
    if kind.x == 11u {
        if kind.w >= 2u { return neutral_frame(in.uv, kind.w == 3u, params.metal.rgb); }
        return unranked(in.uv, kind.w == 1u, params.metal.rgb);
    }
    if kind.w >= 2u {
        return engraving(in.uv, mounted_paint(in.uv), kind, params.metal.rgb, params.gem.rgb, phase);
    }
    let src = textureSample(painting, paint_sampler, in.uv);
    if src.a == 0.0 { return src; }
    let fx = Finish(phase, kind.x, kind.w, kind.y, kind.z, 1u, 0u, 0u);
    return finished_linear(fx, vec4<f32>(srgb(src.rgb), src.a), src, in.uv);
}
