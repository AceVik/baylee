// Smoked obsidian-vellum reading surface: subtle dark vignette, antique-brass
// margin rules, and a smooth page-curl shadow while a chapter leaf turns.
import bevy_ui_render::ui_vertex_output::UiVertexOutput;
@group(1) @binding(0) var<uniform> fold: vec4<f32>;
@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let edge = min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));
    let vignette = exp(-edge * 30.0) * 0.32;
    let fibre = sin(uv.x * 917.0 + sin(uv.y * 331.0)) * sin(uv.y * 811.0) * 0.00025;
    let cloud = sin(uv.x * 9.0 + uv.y * 4.0) * sin(uv.y * 12.0) * 0.00045;
    var vellum = vec3<f32>(0.0040, 0.0076, 0.0128) * (1.0 - vignette) + fibre + cloud;
    let rules = exp(-pow((edge - 0.010) * 1300.0, 2.0)) * 0.18
        + exp(-pow((edge - 0.015) * 1300.0, 2.0)) * 0.07;
    let twig = exp(-pow((edge - 0.024) * 500.0, 2.0))
        * pow(abs(sin((uv.x + uv.y) * 65.0)), 10.0)
        * 0.06;
    vellum = mix(vellum, vec3<f32>(0.42, 0.29, 0.11), clamp(rules + twig, 0.0, 0.35));
    let side = select(uv.x, 1.0 - uv.x, fold.y > 0.0);
    let bend = sin(fold.x * 3.14159265) * fold.z;
    vellum *= 1.0 - bend * (0.22 + pow(side, 5.0) * 0.28);
    vellum += vec3<f32>(0.24, 0.18, 0.08) * pow(side, 14.0) * bend * 0.16;
    return vec4<f32>(max(vellum, vec3<f32>(0.0)), 0.97);
}
