import bevy_ui_render::ui_vertex_output::UiVertexOutput;

@group(1) @binding(0) var<uniform> params: vec4<f32>;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let px = min(in.uv, vec2<f32>(1.0) - in.uv) / max(fwidth(in.uv), vec2<f32>(0.00001));
    let edge = min(px.x, px.y);
    let glint = 0.5 + 0.5 * cos(6.283185 * (in.uv.x * 1.5 - params.x / 6.0));
    let light = (0.14 * params.w + 0.42 * params.y * glint + 0.35 * params.z) * (1.0 - smoothstep(0.5, 3.0, edge));
    return vec4<f32>(1.0, 0.66 + params.z * 0.20, 0.29, clamp(light, 0.0, 0.9));
}
