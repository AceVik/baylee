// Baylee's World (front door) & an enchanted tavern (lobby interior).
// Both scenes stream 16:9 4K 60fps video loops into `sanctuary`.
// All textures are original; live effects never sample card art.
import bevy_ui_render::ui_vertex_output::UiVertexOutput;

struct VistaParams {
    panel: vec4<f32>, home: vec4<f32>, view: vec4<f32>, hour: vec4<f32>,
    gate_a: vec4<f32>, gate_b: vec4<f32>, air: vec4<f32>, portal: vec4<f32>,
}
@group(1) @binding(0) var<uniform> params: VistaParams;
@group(1) @binding(1) var sanctuary: texture_2d<f32>;
@group(1) @binding(2) var sanctuary_sampler: sampler;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let aspect = max(params.view.z, 0.01);
    let energy = params.view.w;
    let interior = params.gate_b.w;
    let flight = params.portal.x * params.portal.y;
    let image_aspect = 16.0 / 9.0;
    let cover = vec2<f32>(min(aspect / image_aspect, 1.0), min(image_aspect / aspect, 1.0));
    let zoom = 1.015 + params.gate_a.w * 0.018 + smoothstep(0.0, 1.0, flight) * 0.06;
    let uv = vec2<f32>(0.5)
        + (in.uv - 0.5) * cover / zoom
        + params.view.xy * vec2<f32>(0.0014, 0.0009) * energy;
    let sample_uv = clamp(uv, vec2<f32>(0.001), vec2<f32>(0.999));
    var rgb = textureSample(sanctuary, sanctuary_sampler, sample_uv).rgb;

    // Soft reading contrast & passage haze on the front door login form:
    let p = in.uv * vec2<f32>(aspect, 1.0);
    if interior <= 0.5 {
        let panel_q = abs(p - params.panel.xy) - params.panel.zw;
        let quiet = 1.0 - smoothstep(0.0, 0.12, length(max(panel_q, vec2<f32>(0.0))));
        rgb *= 1.0 - quiet * 0.18 * (1.0 - params.portal.y);
        let vignette = smoothstep(0.42, 0.85, length(in.uv - 0.5));
        rgb *= 1.0 - vignette * 0.05;
        rgb = mix(rgb, vec3<f32>(0.91, 0.88, 0.79), params.hour.y * 0.12);
    }

    var alpha = params.air.z;
    if params.portal.y > 0.5 {
        let radius = length(p - vec2<f32>(aspect * 0.5, 0.48));
        let reveal = smoothstep(0.35, 1.0, flight) * (length(vec2<f32>(aspect, 1.0)) + 0.3);
        alpha *= smoothstep(reveal - 0.3, reveal + 0.1, radius);
    }
    return vec4<f32>(rgb, alpha);
}
