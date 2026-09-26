// Baylee's sanctuary: original painted world, separately lit foreground,
// analytic fireflies, drifting mist and reflected moonlight. All movement
// reads virtual seconds; reduced motion freezes both time and pointer.
#import bevy_ui::ui_vertex_output::UiVertexOutput
#import "embedded://baylee_client/shaders/noise.wgsl"::{noise2, hash2, fbm2}

struct VistaParams {
    panel: vec4<f32>,
    view: vec4<f32>,
    hour: vec4<f32>,
    gate_a: vec4<f32>,
    gate_b: vec4<f32>,
    air: vec4<f32>,
    portal: vec4<f32>,
}
@group(1) @binding(0) var<uniform> params: VistaParams;
@group(1) @binding(1) var sanctuary: texture_2d<f32>;
@group(1) @binding(2) var sanctuary_sampler: sampler;
@group(1) @binding(3) var guardian: texture_2d<f32>;
@group(1) @binding(4) var guardian_sampler: sampler;
@group(1) @binding(5) var lantern: texture_2d<f32>;
@group(1) @binding(6) var lantern_sampler: sampler;
@group(1) @binding(7) var arch: texture_2d<f32>;
@group(1) @binding(8) var arch_sampler: sampler;

const GOLD: vec3<f32> = vec3<f32>(0.9, 0.49, 0.13);
const MOON: vec3<f32> = vec3<f32>(0.15, 0.38, 0.52);
const TAU: f32 = 6.2831853;

fn bell(p: vec2<f32>, size: vec2<f32>) -> f32 {
    let q = p / size;
    return exp(-dot(q, q));
}

// A local neighbourhood avoids looping over every particle on every pixel.
// Different scales, phases, depths and velocities prevent a visible grid.
fn fireflies(p: vec2<f32>, t: f32, scale: f32, depth: f32, rush: f32) -> vec3<f32> {
    let focus = vec2<f32>(params.view.z * 0.5, 0.48);
    let radial = p - focus;
    let travel = 1.0 + rush * 3.0;
    let q = (focus + radial * travel + vec2<f32>(t * 0.011 * depth, t * 0.016)) * scale;
    let cell = floor(q);
    var light = 0.0;
    for (var y = -1; y <= 1; y = y + 1) {
        for (var x = -1; x <= 1; x = x + 1) {
            let id = cell + vec2<f32>(f32(x), f32(y));
            let seed = hash2(id + depth * 7.3);
            let alive = smoothstep(0.48, 0.7, seed);
            let phase = seed * TAU;
            let pos = id + vec2<f32>(0.5 + 0.30 * sin(t * 0.29 + phase),
                0.5 + 0.24 * cos(t * 0.21 + phase * 3.0));
            let d = length((q - pos) * vec2<f32>(1.0, 1.0 + rush * 3.0));
            let pulse = 0.35 + 0.65 * pow(0.5 + 0.5 * sin(t * 0.8 + phase), 2.0);
            light += (exp(-d * d * 1300.0) * 0.55 + exp(-d * d * 65.0) * 0.025) * pulse * alive;
        }
    }
    return mix(GOLD, MOON * 1.7, smoothstep(0.5, 1.1, depth)) * light;
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let aspect = max(params.view.z, 0.01);
    let t = params.portal.z;
    let energy = params.view.w;
    let pointer = params.view.xy * energy;
    let flight = params.portal.x * params.portal.y;
    let advance = smoothstep(0.08, 0.92, flight);
    let p = in.uv * vec2<f32>(aspect, 1.0);
    let focus = vec2<f32>(aspect * 0.5, 0.48);
    let zoom = 1.035 + params.gate_a.w * 0.065 + advance * advance * 1.8;
    // Cover, never stretch: the architecture keeps its proportions in portrait.
    let image_aspect = 1672.0 / 941.0;
    let cover = vec2<f32>(min(aspect / image_aspect, 1.0), min(image_aspect / aspect, 1.0));

    let drift = vec2<f32>(sin(t * 0.073), cos(t * 0.091)) * 0.0012 * energy;
    var uv = vec2<f32>(0.5) + (in.uv - vec2<f32>(0.5)) * cover / zoom;
    uv += pointer * vec2<f32>(0.003, 0.002) + drift * 0.3;
    // Only the water moves: no full-screen wobble of stone or distant towers.
    let stream = bell(uv - vec2<f32>(0.46, 0.87), vec2<f32>(0.20, 0.10));
    uv.x += sin(uv.y * 520.0 - t * 1.4) * 0.0007 * stream * energy;
    let sky_drift = (1.0 - smoothstep(0.16, 0.27, uv.y))
        * (1.0 - bell(uv - vec2<f32>(0.758, 0.06), vec2<f32>(0.045, 0.06)));
    let sky_uv = uv + vec2<f32>(sin(t * 0.022) * 0.012, cos(t * 0.017) * 0.002) * sky_drift * energy;
    var rgb = textureSample(sanctuary, sanctuary_sampler, sky_uv).rgb;
    rgb *= 0.88 + 0.06 * params.hour.x;

    // Broad angled shafts from the moon; branch-like breaks in their light.
    let ray_axis = uv.x - 0.76 + (uv.y - 0.08) * 0.34;
    let ray = pow(max(0.0, sin(ray_axis * 62.0 + 0.12 * sin(t * 0.14))), 12.0);
    let ray_mask = smoothstep(0.12, 0.35, uv.y) * (1.0 - smoothstep(0.65, 0.9, uv.y));
    rgb += MOON * ray * ray_mask * 0.035;
    // Wisps move at two speeds and dissolve at their edges, not a scrolling overlay.
    let fog_a = noise2(uv * vec2<f32>(8.0, 19.0) + vec2<f32>(t * 0.021, -t * 0.006));
    let fog_b = noise2(uv * vec2<f32>(15.0, 29.0) - vec2<f32>(t * 0.012, t * 0.004));
    let mist = pow(fog_a * 0.65 + fog_b * 0.35, 2.0)
        * bell(uv - vec2<f32>(0.48, 0.68), vec2<f32>(0.40, 0.15));
    rgb = mix(rgb, MOON * 0.65, mist * 0.13);
    let shimmer = pow(noise2(vec2<f32>(uv.x * 240.0, uv.y * 450.0 - t * 0.48)), 7.0);
    rgb += MOON * shimmer * stream * params.air.x * 0.32;
    // Small warm pools anchored to the painted hanging lamps.
    let flicker = 0.92 + 0.05 * sin(t * 2.1) + 0.03 * sin(t * 3.7);
    rgb += GOLD * (bell(uv - vec2<f32>(0.115, 0.17), vec2<f32>(0.029, 0.046))
        + bell(uv - vec2<f32>(0.925, 0.185), vec2<f32>(0.027, 0.047))) * flicker * 0.065;

    // Sky vapour crosses the moon on its own plane; scattered light follows it.
    let sky = 1.0 - smoothstep(0.16, 0.38, uv.y);
    let cloud_at = uv * vec2<f32>(9.0, 16.0) + vec2<f32>(t * 0.013, t * 0.002);
    let cloud = smoothstep(0.30, 0.72, fbm2(cloud_at
        + vec2<f32>(fbm2(cloud_at * 0.6 + 17.0), fbm2(cloud_at * 0.7 - 9.0)) * 2.0));
    rgb = mix(rgb, vec3<f32>(0.065, 0.115, 0.17), cloud * sky * 0.18);
    let moon_halo = bell(uv - vec2<f32>(0.758, 0.060), vec2<f32>(0.07, 0.12));
    rgb += MOON * moon_halo * (0.11 + 0.015 * sin(t * 0.25)) * (1.0 - cloud * 0.7);
    // Narrow falling streams and expanding ripples stay inside water masks.
    let falls = bell(uv - vec2<f32>(0.234, 0.510), vec2<f32>(0.007, 0.065))
        + bell(uv - vec2<f32>(0.451, 0.654), vec2<f32>(0.006, 0.036))
        + bell(uv - vec2<f32>(0.720, 0.604), vec2<f32>(0.005, 0.026))
        + bell(uv - vec2<f32>(0.748, 0.750), vec2<f32>(0.007, 0.05));
    let cascade = noise2(vec2<f32>(uv.x * 1200.0, uv.y * 200.0 - t * 3.0));
    rgb += vec3<f32>(0.19, 0.32, 0.38) * falls * cascade * 0.34;
    let ripples = pow(max(0.0, sin(uv.y * 600.0 - t * 2.5
        + noise2(uv * 32.0) * 9.0)), 12.0);
    rgb += MOON * stream * ripples * 0.07;

    // The near arch moves as rigid architecture, at four times the distant
    // parallax. Advancing through it carries it beyond the screen edges.
    let arch_zoom = zoom + advance * 2.2;
    let arch_uv = vec2<f32>(0.5) + (in.uv - vec2<f32>(0.5)) * cover / arch_zoom
        + pointer * vec2<f32>(0.014, 0.009) + drift;
    let stone = textureSample(arch, arch_sampler, arch_uv);
    let lamp_light = bell(arch_uv - vec2<f32>(0.12, 0.16), vec2<f32>(0.03, 0.05))
        + bell(arch_uv - vec2<f32>(0.925, 0.165), vec2<f32>(0.03, 0.05));
    rgb = mix(rgb, stone.rgb * 0.92 + GOLD * lamp_light * flicker * 0.10, stone.a);

    // Foreground props are kept outside the form. On narrow screens the
    // guardian remains in the identity above it, rather than covering controls.
    let wide = smoothstep(1.20, 1.55, aspect);
    let foreground = wide * (1.0 - smoothstep(0.02, 0.55, flight));
    let ground = min(0.92, 1.0 - 105.0 / max(params.portal.w, 320.0));
    let cat_foot = vec2<f32>(aspect * 0.235, ground) - pointer * 0.012;
    let cat_height = 0.285 * (1.0 + 0.0035 * sin(t * 1.55));
    let cat_size = vec2<f32>(cat_height * 0.75, cat_height);
    let cat_uv = (p - cat_foot) / cat_size + vec2<f32>(0.5, 1.0);
    let cat = textureSample(guardian, guardian_sampler, clamp(cat_uv, vec2<f32>(0.0), vec2<f32>(1.0)));
    let cat_inside = step(0.0, cat_uv.x) * step(cat_uv.x, 1.0) * step(0.0, cat_uv.y) * step(cat_uv.y, 1.0);
    let shadow = bell(p - cat_foot + vec2<f32>(0.0, 0.014), vec2<f32>(0.106, 0.014));
    rgb *= 1.0 - shadow * 0.65 * foreground;
    // The sprite's painted moonlight remains cool; the nearby lantern warms its foot.
    rgb = mix(rgb, cat.rgb * vec3<f32>(0.62, 0.69, 0.77), cat.a * cat_inside * foreground);

    let lamp_foot = vec2<f32>(aspect * 0.15, ground + 0.02) - pointer * 0.018;
    let lamp_uv = (p - lamp_foot) / vec2<f32>(0.139, 0.185) + vec2<f32>(0.5, 1.0);
    let lamp = textureSample(lantern, lantern_sampler, clamp(lamp_uv, vec2<f32>(0.0), vec2<f32>(1.0)));
    let lamp_inside = step(0.0, lamp_uv.x) * step(lamp_uv.x, 1.0) * step(0.0, lamp_uv.y) * step(lamp_uv.y, 1.0);
    rgb *= 1.0 - 0.55 * foreground * bell(p - lamp_foot + vec2<f32>(0.0, 0.012), vec2<f32>(0.055, 0.012));
    rgb += GOLD * bell(p - lamp_foot + vec2<f32>(0.0, 0.075), vec2<f32>(0.082, 0.12)) * 0.08 * foreground * flicker;
    rgb = mix(rgb, lamp.rgb * 0.8 * flicker, lamp.a * lamp_inside * foreground);

    // Quiet space behind the form and footer, without dimming the whole vista.
    let panel_q = abs(p - params.panel.xy) - params.panel.zw;
    let outside = length(max(panel_q, vec2<f32>(0.0)));
    let quiet = 1.0 - smoothstep(0.0, 0.085, outside);
    rgb *= 1.0 - quiet * 0.55 * (1.0 - params.portal.y);
    let edge = smoothstep(0.35, 0.78, length(in.uv - vec2<f32>(0.5)));
    rgb *= 1.0 - edge * 0.32;


    let air_mask = smoothstep(0.18, 0.40, in.uv.y) * (1.0 - smoothstep(0.85, 1.0, in.uv.y));
    var motes = fireflies(p + pointer * 0.025, t, 11.0, 0.45, advance);
    if params.hour.w > 0.5 {
        motes += fireflies(p - pointer * 0.013, t + 73.0, 18.0, 1.2, advance) * 0.55;
    }
    rgb += motes * air_mask * (1.0 - quiet * (1.0 - params.portal.y));
    rgb = mix(rgb, MOON * 0.55, params.hour.y * 0.28);

    // Successful authentication accelerates into the painted world. Light
    // streaks converge at the destination; a feathered aperture reveals the
    // actual already-loaded lobby, ending fully transparent at all aspects.
    var opacity = params.air.z;
    if params.portal.y > 0.5 {
        let r = length(p - focus);
        let theta = atan2(p.y - focus.y, p.x - focus.x);
        let streak = pow(max(0.0, sin(theta * 43.0 + noise2(vec2<f32>(theta * 5.0, 0.0)) * 6.0)), 24.0);
        let charge = smoothstep(0.0, 0.3, flight) * (1.0 - smoothstep(0.75, 1.0, flight));
        rgb += mix(MOON, GOLD, 0.4) * streak * smoothstep(0.05, 0.6, r) * charge * 0.16;
        let reveal = smoothstep(0.55, 1.0, flight) * (length(vec2<f32>(aspect, 1.0)) + 0.2);
        opacity *= smoothstep(reveal - 0.12, reveal + 0.04, r);
        rgb += MOON * exp(-abs(r - reveal) * 48.0) * charge * 0.16;
    }
    return vec4<f32>(rgb, opacity);
}
