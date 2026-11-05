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

    // Spatial masks for the painting's natural elements (uniform control flow):
    // 1. Turquoise city river/canals winding through the valley floor.
    let river_upper = bell(uv - vec2<f32>(0.44, 0.63), vec2<f32>(0.13, 0.11));
    let river_lower = bell(uv - vec2<f32>(0.48, 0.81), vec2<f32>(0.17, 0.14));
    let river_zone = clamp(river_upper + river_lower, 0.0, 1.0);
    // 2. Distant sunlit ocean bay on the left horizon.
    let ocean_zone = bell(uv - vec2<f32>(0.15, 0.54), vec2<f32>(0.17, 0.07));
    // 3. Cliff waterfalls cascading on the left and right canyon walls.
    let falls_left = bell(uv - vec2<f32>(0.32, 0.56), vec2<f32>(0.045, 0.095));
    let falls_right = bell(uv - vec2<f32>(0.57, 0.50), vec2<f32>(0.045, 0.085));
    let falls_zone = clamp(falls_left + falls_right, 0.0, 1.0);
    // 4. Billowing cloud sea on the left and upper-left sky.
    let cloud_sea = bell(uv - vec2<f32>(0.20, 0.34), vec2<f32>(0.26, 0.18));

    // Gentle UV refraction for water ripples, cascading waterfalls, and cloud drift.
    let ripple_a = sin(uv.y * 190.0 - t * 2.4 + uv.x * 45.0)
        * cos(uv.x * 130.0 + t * 1.7 - uv.y * 60.0);
    let ripple_b = sin(uv.y * 310.0 + t * 3.1 - uv.x * 85.0);
    let water_disp = vec2<f32>(ripple_a * 0.0014, ripple_b * 0.0007)
        * (river_zone + ocean_zone * 0.65)
        * energy;

    let fall_flow = noise2(vec2<f32>(uv.x * 55.0, uv.y * 28.0 - t * 1.85)) - 0.5;
    let falls_disp = vec2<f32>(fall_flow * 0.0008, fall_flow * 0.0022) * falls_zone * energy;

    let cloud_drift = vec2<f32>(
        noise2(uv * vec2<f32>(6.0, 10.0) + vec2<f32>(-t * 0.035, t * 0.012)) - 0.5,
        noise2(uv * vec2<f32>(8.0, 12.0) + vec2<f32>(t * 0.022, -t * 0.015)) - 0.5,
    ) * 0.0024 * cloud_sea * energy;

    let sample_uv = clamp(uv + water_disp + falls_disp + cloud_drift, vec2<f32>(0.001), vec2<f32>(0.999));
    var rgb = textureSample(sanctuary, sanctuary_sampler, sample_uv).rgb;

    // Sparkling water caustics & sunlit wave crests, gated by the turquoise water hue
    // so bridges and stone embankments stay crisp.
    let aqua = clamp((rgb.g * 0.55 + rgb.b * 0.65) - rgb.r * 0.85, 0.0, 1.0);
    let water_mask = clamp((river_zone + ocean_zone * 0.85) * smoothstep(0.04, 0.22, aqua), 0.0, 1.0);
    if water_mask > 0.002 {
        let c1 = noise2(uv * vec2<f32>(42.0, 88.0) + vec2<f32>(t * 0.42, -t * 0.31));
        let c2 = noise2(uv * vec2<f32>(68.0, 132.0) + vec2<f32>(-t * 0.36, t * 0.48));
        let shimmer = smoothstep(0.60, 0.88, (c1 + c2) * 0.5);
        let crest = pow(max(0.0, sin(uv.y * 240.0 - t * 2.8 + c1 * 4.5)), 6.0);
        let glint = (shimmer * 0.11 + crest * 0.06) * water_mask * energy;
        rgb += vec3<f32>(0.68, 0.96, 1.0) * glint;
    }

    // Waterfall cascading foam & rising mist at the base of the falls.
    if falls_zone > 0.002 {
        let stream = noise2(vec2<f32>(uv.x * 72.0, uv.y * 34.0 - t * 2.2));
        let spray = smoothstep(0.52, 0.86, stream) * falls_zone * 0.11 * energy;
        rgb = mix(rgb, vec3<f32>(0.92, 0.97, 1.0), spray);
    }

    // Multi-layered drifting cloud veils between the cloud bank, horizon, and valley.
    let cloud_band_a = bell(uv - vec2<f32>(0.26, 0.44), vec2<f32>(0.42, 0.15));
    let cloud_band_b = bell(uv - vec2<f32>(0.42, 0.36), vec2<f32>(0.34, 0.12));
    let cloud_band = clamp(cloud_band_a + cloud_band_b * 0.75, 0.0, 1.0);
    if cloud_band > 0.002 {
        let shape_a = noise2(uv * vec2<f32>(6.5, 15.0) + vec2<f32>(t * 0.024, t * 0.005));
        let shape_b = noise2(uv * vec2<f32>(13.0, 26.0) + vec2<f32>(-t * 0.016, t * 0.008));
        let shape = shape_a * 0.65 + shape_b * 0.35;
        let veil = smoothstep(0.36, 0.80, shape) * cloud_band * 0.15;
        rgb = mix(rgb, vec3<f32>(0.95, 0.93, 0.89), veil);
    }

    // Sunbeams & warm golden light motes spreading from the upper-right sun.
    let sun = vec2<f32>(0.983, 0.065);
    let radial = (uv - sun) * vec2<f32>(image_aspect, 1.0);
    let angle = atan2(radial.y, radial.x);
    let shafts_a = pow(max(0.0, sin(angle * 35.0 + sin(t * 0.14) * 0.22)), 12.0);
    let shafts_b = pow(max(0.0, sin(angle * 21.0 - cos(t * 0.11) * 0.18 + 1.1)), 10.0);
    let shafts = shafts_a * 0.65 + shafts_b * 0.35;
    let beam = bell(uv - sun, vec2<f32>(0.68, 0.72)) * smoothstep(0.10, 0.42, length(radial));
    rgb += vec3<f32>(1.0, 0.84, 0.56) * shafts * beam * 0.036;
    rgb += vec3<f32>(1.0, 0.89, 0.68) * bell(uv - sun, vec2<f32>(0.11, 0.13)) * (0.028 + 0.006 * sin(t * 0.8) * energy);

    let mote_field = noise2(uv * vec2<f32>(38.0, 38.0) + vec2<f32>(-t * 0.08, -t * 0.12));
    let mote_spark = smoothstep(0.84, 0.95, mote_field) * beam * 0.07 * energy;
    rgb += vec3<f32>(1.0, 0.92, 0.72) * mote_spark;

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
