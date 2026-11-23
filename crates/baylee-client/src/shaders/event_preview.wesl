import bevy_render::globals::Globals;
import bevy_ui_render::ui_vertex_output::UiVertexOutput;
import baylee_client::shaders::card_common::varnish;

@group(0) @binding(1) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> settings: vec4<f32>;
@group(1) @binding(1) var art: texture_2d<f32>;
@group(1) @binding(2) var art_sampler: sampler;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let size = max(in.size, vec2<f32>(1.0));
    // A reward preview shows the whole piece even in a narrow inventory cell.
    let pixels = vec2<f32>(textureDimensions(art));
    let ratio = (size.x / size.y) * pixels.y / max(pixels.x, 1.0);
    let scale = max(vec2<f32>(1.0), vec2<f32>(ratio, 1.0 / max(ratio, 0.001)));
    let uv = vec2<f32>(0.5) + (in.uv - vec2<f32>(0.5)) * scale;
    let paper = textureSample(art, art_sampler, clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0)));
    let canvas = all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0));
    let r = min(in.border_radius_x.x, min(size.x, size.y) * 0.5);
    let q = abs((in.uv - vec2<f32>(0.5)) * size) - size * 0.5 + vec2<f32>(r);
    let edge = length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
    return vec4<f32>(varnish(paper.rgb, uv, globals.time * settings.y, u32(settings.x)), select(0.0, paper.a * clamp(0.5 - edge, 0.0, 1.0), canvas));
}
