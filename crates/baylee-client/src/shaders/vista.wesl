// Baylee's World: a garden city under a sunlit cloud horizon.
// The image is original; live cloud veil and sunlight never sample card art.
import bevy_ui_render::ui_vertex_output::UiVertexOutput;
import baylee_client::shaders::noise::{noise2};
struct VistaParams {
    panel: vec4<f32>, home: vec4<f32>, view: vec4<f32>, hour: vec4<f32>,
    gate_a: vec4<f32>, gate_b: vec4<f32>, air: vec4<f32>, portal: vec4<f32>,
}
@group(1) @binding(0) var<uniform> params: VistaParams;
@group(1) @binding(1) var sanctuary: texture_2d<f32>;
@group(1) @binding(2) var sanctuary_sampler: sampler;

fn bell(at: vec2<f32>, size: vec2<f32>) -> f32 {
    let q = at / size;
    return exp(-dot(q, q));
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let aspect = max(params.view.z, 0.01);
    let t = params.portal.z;
    let energy = params.view.w;
    let interior = params.gate_b.w;
    let flight = params.portal.x * params.portal.y;
    let image_aspect = 1672.0 / 941.0;
    let cover = vec2<f32>(min(aspect / image_aspect, 1.0), min(image_aspect / aspect, 1.0));
    let zoom = 1.015 + params.gate_a.w * 0.018 + smoothstep(0.0, 1.0, flight) * 0.06;
    var uv = vec2<f32>(0.5) + (in.uv - 0.5) * cover / zoom;
    uv += params.view.xy * vec2<f32>(0.0017, 0.001) * energy;
    var rgb = textureSample(sanctuary, sanctuary_sampler, uv).rgb;

    // Low-frequency cloud veils drift between the near cloud bank and city.
    // Only the distant horizon receives them: all architecture stays sharp.
    let cloud_band = bell(uv - vec2<f32>(0.28, 0.46), vec2<f32>(0.40, 0.13));
    if cloud_band > 0.002 {
        let shape = noise2(uv * vec2<f32>(7.0, 17.0) + vec2<f32>(t * 0.009, t * 0.002));
        let veil = smoothstep(0.38, 0.82, shape) * cloud_band * 0.12;
        rgb = mix(rgb, vec3<f32>(0.93, 0.92, 0.88), veil);
    }
    // Rays spread from the actual sun in the upper right of the painting.
    let sun = vec2<f32>(0.983, 0.065);
    let radial = (uv - sun) * vec2<f32>(image_aspect, 1.0);
    let angle = atan2(radial.y, radial.x);
    let shafts = pow(max(0.0, sin(angle * 35.0 + sin(t * 0.075) * 0.12)), 14.0);
    let beam = bell(uv - sun, vec2<f32>(0.64, 0.69)) * smoothstep(0.12, 0.4, length(radial));
    rgb += vec3<f32>(1.0, 0.82, 0.54) * shafts * beam * 0.025;
    rgb += vec3<f32>(1.0, 0.88, 0.66) * bell(uv - sun, vec2<f32>(0.1, 0.12)) * 0.025;

    // A soft reading shadow follows the existing form, not the whole sky.
    let p = in.uv * vec2<f32>(aspect, 1.0);
    let panel_q = abs(p - params.panel.xy) - params.panel.zw;
    let quiet = 1.0 - smoothstep(0.0, 0.12, length(max(panel_q, vec2<f32>(0.0))));
    rgb *= 1.0 - quiet * (1.0 - interior) * 0.22 * (1.0 - params.portal.y);
    rgb *= 1.0 - interior * 0.12;
    let vignette = smoothstep(0.38, 0.80, length(in.uv - 0.5));
    rgb *= 1.0 - vignette * 0.08;
    rgb = mix(rgb, vec3<f32>(0.91, 0.88, 0.79), params.hour.y * 0.12);

    // Arrival is a quiet passage through light; the real lobby is underneath.
    var alpha = params.air.z;
    if params.portal.y > 0.5 {
        let radius = length(p - vec2<f32>(aspect * 0.5, 0.48));
        let reveal = smoothstep(0.35, 1.0, flight) * (length(vec2<f32>(aspect, 1.0)) + 0.3);
        alpha *= smoothstep(reveal - 0.3, reveal + 0.1, radius);
    }
    return vec4<f32>(rgb, alpha);
}
