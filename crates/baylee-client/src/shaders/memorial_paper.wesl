// Original analytic paper: stationary fibres, gold rules and a moving leaf shadow.
import bevy_ui_render::ui_vertex_output::UiVertexOutput;
@group(1) @binding(0) var<uniform> fold: vec4<f32>;
@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let edge = min(min(uv.x, 1.0-uv.x), min(uv.y, 1.0-uv.y));
    let stain = exp(-edge * 38.0) * 0.11;
    let fibre = sin(uv.x * 917.0 + sin(uv.y * 331.0)) * sin(uv.y * 811.0) * 0.003;
    let cloud = sin(uv.x * 9.0 + uv.y * 4.0) * sin(uv.y * 12.0) * 0.008;
    var paper = vec3<f32>(0.962, 0.919, 0.814) - stain * vec3<f32>(0.60, 0.65, 0.70) + fibre + cloud;
    let rules = exp(-pow((edge-0.011)*1300.0, 2.0)) * 0.4 + exp(-pow((edge-0.016)*1300.0,2.0))*0.15;
    // Sparse gold leaves live only in the margins, never beneath a sentence.
    let twig = exp(-pow((edge-0.025)*500.0,2.0)) * pow(abs(sin((uv.x+uv.y)*65.0)), 10.0) * 0.15;
    paper = mix(paper, vec3<f32>(0.60, 0.43, 0.22), rules + twig);
    let side = select(uv.x, 1.0-uv.x, fold.y > 0.0);
    let bend = sin(fold.x * 3.14159265) * fold.z;
    paper *= 1.0 - bend * (0.12 + pow(side, 5.0) * 0.20);
    paper += pow(side, 15.0) * bend * 0.14;
    return vec4<f32>(paper, 1.0);
}
