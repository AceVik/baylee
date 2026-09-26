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

// Continuous 3D value field: adjacent height slices share the same lattice.
fn vapour(p: vec3<f32>) -> f32 {
    let z = floor(p.z);
    let f = fract(p.z);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(noise2(p.xy + z * vec2<f32>(17.0, 31.0)),
        noise2(p.xy + (z + 1.0) * vec2<f32>(17.0, 31.0)), u);
}
fn cloud_density(p: vec3<f32>) -> f32 {
    return smoothstep(0.42, 0.72, vapour(p) * 0.58
        + vapour(p * 2.03 + 9.0) * 0.28 + vapour(p * 4.11 - 7.0) * 0.14);
}
// View rays cross a finite horizontal cloud volume. Front-to-back integration
// gives real depth/occlusion and wind-driven movement rather than sliding sky pixels.
fn cloud_volume(uv: vec2<f32>, eye: vec2<f32>, t: f32) -> vec4<f32> {
    let ray = normalize(vec3<f32>((uv.x - 0.5) * 1.777, 0.46 - uv.y, -1.1));
    let origin = vec3<f32>(eye.x * 0.3, 0.0, eye.y * 0.15);
    var color = vec3<f32>(0.0);
    var opacity = 0.0;
    let count = select(3.0, 6.0, params.hour.w > 0.5);
    for (var i = 0; i < 6; i = i + 1) {
        if f32(i) >= count { break; }
        let h = 2.3 + (f32(i) + 0.5) * 1.6 / count;
        let hit = origin + ray * (h / max(ray.y, 0.08));
        let at = hit * vec3<f32>(0.31, 0.95, 0.31) + vec3<f32>(t * 0.035, 0.0, t * 0.017);
        let density = cloud_density(at);
        let lit = clamp(1.0 - cloud_density(at + vec3<f32>(0.15, 0.3, 0.0)), 0.0, 1.0);
        let a = (1.0 - exp(-density * 1.1 / count));
        let moon = bell(uv - vec2<f32>(0.758, 0.060), vec2<f32>(0.23, 0.26));
        let light = mix(vec3<f32>(0.025, 0.047, 0.071), vec3<f32>(0.14, 0.22, 0.29), lit * (0.35 + moon * 0.65));
        color += light * a * (1.0 - opacity);
        opacity += a * (1.0 - opacity);
    }
    let sky = 1.0 - smoothstep(0.22, 0.42, uv.y);
    return vec4<f32>(color * sky, opacity * sky);
}
fn river_mask(uv: vec2<f32>) -> f32 {
    let bank = mix(0.11, 0.34, smoothstep(0.78, 1.0, uv.y));
    var mask = (1.0 - smoothstep(bank - 0.025, bank + 0.025, abs(uv.x - 0.49)))
        * smoothstep(0.78, 0.83, uv.y);
    mask *= 1.0 - bell(uv - vec2<f32>(0.345, 0.845), vec2<f32>(0.075, 0.038));
    mask *= 1.0 - bell(uv - vec2<f32>(0.68, 0.87), vec2<f32>(0.10, 0.06));
    return mask;
}
// Analytic derivatives of four travelling waves on a horizontal world plane.
// Their wavelength shrinks in the distance through perspective, not a UV trick.
fn water_height(at: vec2<f32>, t: f32) -> f32 {
    return sin(dot(at, vec2<f32>(2.8, 1.2)) - t * 1.25) * 0.022
        + sin(dot(at, vec2<f32>(-1.4, 4.1)) - t * 1.8) * 0.012
        + sin(dot(at, vec2<f32>(6.9, 3.5)) - t * 2.6) * 0.005
        + sin(dot(at, vec2<f32>(-9.2, 7.7)) - t * 3.3) * 0.0025;
}
fn water_normal(at: vec2<f32>, t: f32) -> vec3<f32> {
    let a = dot(at, vec2<f32>(2.8, 1.2)) - t * 1.25;
    let b = dot(at, vec2<f32>(-1.4, 4.1)) - t * 1.8;
    let c = dot(at, vec2<f32>(6.9, 3.5)) - t * 2.6;
    let d = dot(at, vec2<f32>(-9.2, 7.7)) - t * 3.3;
    let slope = vec2<f32>(2.8, 1.2) * cos(a) * 0.022
        + vec2<f32>(-1.4, 4.1) * cos(b) * 0.012
        + vec2<f32>(6.9, 3.5) * cos(c) * 0.005
        + vec2<f32>(-9.2, 7.7) * cos(d) * 0.0025;
    return normalize(vec3<f32>(-slope.x, 1.0, -slope.y));
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
    let interior = params.gate_b.w;
    let pointer = params.view.xy * energy * (1.0 - interior * 0.55);
    let flight = params.portal.x * params.portal.y;
    let advance = smoothstep(0.08, 0.92, flight);
    let p = in.uv * vec2<f32>(aspect, 1.0);
    let focus = vec2<f32>(aspect * 0.5, 0.48);
    let zoom = 1.035 + params.gate_a.w * 0.065 + advance * advance * 1.8 + interior * 0.10;
    // Cover, never stretch: the architecture keeps its proportions in portrait.
    let image_aspect = 1672.0 / 941.0;
    let cover = vec2<f32>(min(aspect / image_aspect, 1.0), min(image_aspect / aspect, 1.0));

    let drift = vec2<f32>(sin(t * 0.073), cos(t * 0.091)) * 0.0012 * energy;
    var uv = vec2<f32>(0.5) + (in.uv - vec2<f32>(0.5)) * cover / zoom;
    uv += pointer * vec2<f32>(0.003, 0.002) + drift * 0.3;
    let world_uv = uv;
    let stream = river_mask(uv);
    let eye = vec3<f32>(pointer.x * 0.24, 1.4, 2.8 + pointer.y * 0.10);
    let water_ray = normalize(vec3<f32>((uv.x - 0.5) * image_aspect, 0.70 - uv.y, -1.15));
    var water_hit = eye + water_ray * (eye.y / max(-water_ray.y, 0.035));
    if stream > 0.001 {
        for (var iteration = 0; iteration < 2; iteration = iteration + 1) {
            water_hit = eye + water_ray * ((eye.y - water_height(water_hit.xz, t)) / max(-water_ray.y, 0.035));
        }
    }
    let normal = water_normal(water_hit.xz, t);
    uv += normal.xz * vec2<f32>(0.013, 0.006) * stream;
    var rgb = textureSample(sanctuary, sanctuary_sampler, uv).rgb;
    let reflection_uv = vec2<f32>(uv.x + normal.x * 0.018,
        0.77 - (uv.y - 0.77) * 0.65 + normal.z * 0.012);
    let reflection = textureSample(sanctuary, sanctuary_sampler, reflection_uv).rgb;
    let view_dir = normalize(eye - water_hit);
    let fresnel = 0.05 + 0.42 * pow(1.0 - max(dot(normal, view_dir), 0.0), 5.0);
    rgb = mix(rgb, reflection * vec3<f32>(0.55, 0.75, 0.90), stream * fresnel);
    let half_light = normalize(view_dir + normalize(vec3<f32>(0.25, 0.65, -0.20)));
    let silver = pow(max(dot(normal, half_light), 0.0), 56.0);
    let moon_path = bell(world_uv - vec2<f32>(0.55, 0.92), vec2<f32>(0.14, 0.22));
    rgb += vec3<f32>(0.32, 0.47, 0.56) * silver * stream * moon_path * 0.65;
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

    // The painted moon retains its craters; a luminous disc, aureole and
    // scattering are occluded together by the cloud volume in front of it.
    let moon_at = (world_uv - vec2<f32>(0.758, 0.060)) * vec2<f32>(image_aspect, 1.0);
    let moon_r = length(moon_at);
    let moon_halo = exp(-moon_r * 24.0);
    let disc = 1.0 - smoothstep(0.026, 0.029, moon_r);
    rgb += vec3<f32>(0.23, 0.34, 0.40) * moon_halo * 0.18;
    rgb += vec3<f32>(0.34, 0.42, 0.43) * disc * 0.12;
    let clouds = cloud_volume(world_uv, pointer, t);
    rgb = rgb * (1.0 - clouds.a) + clouds.rgb;
    // Narrow falling streams and expanding ripples stay inside water masks.
    let falls = bell(uv - vec2<f32>(0.234, 0.510), vec2<f32>(0.007, 0.065))
        + bell(uv - vec2<f32>(0.451, 0.654), vec2<f32>(0.006, 0.036))
        + bell(uv - vec2<f32>(0.720, 0.604), vec2<f32>(0.005, 0.026))
        + bell(uv - vec2<f32>(0.748, 0.750), vec2<f32>(0.007, 0.05));
    let cascade = pow(noise2(vec2<f32>(uv.x * 950.0, uv.y * 125.0 - t * 2.9)), 2.0)
        + 0.5 * pow(max(0.0, sin(uv.y * 510.0 - t * 13.0 + uv.x * 32.0)), 6.0);
    rgb += vec3<f32>(0.19, 0.32, 0.38) * falls * cascade * 0.62;
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
    let foreground = (1.0 - interior) * wide * (1.0 - smoothstep(0.02, 0.55, flight));
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
    let quiet = (1.0 - smoothstep(0.0, 0.085, outside)) * (1.0 - interior);
    rgb *= 1.0 - quiet * 0.55 * (1.0 - params.portal.y);
    let edge = smoothstep(0.35, 0.78, length(in.uv - vec2<f32>(0.5)));
    rgb *= 1.0 - edge * 0.32;


    let air_mask = smoothstep(0.18, 0.40, in.uv.y) * (1.0 - smoothstep(0.85, 1.0, in.uv.y));
    var motes = fireflies(p + pointer * 0.025, t, 11.0, 0.45, advance);
    if params.hour.w > 0.5 {
        motes += fireflies(p - pointer * 0.013, t + 73.0, 18.0, 1.2, advance) * 0.55;
    }
    rgb += motes * air_mask * (1.0 - quiet * (1.0 - params.portal.y)) * (1.0 - interior * 0.5);
    rgb = mix(rgb, MOON * 0.55, params.hour.y * 0.28);
    // The editor lives in the same garden, with a calm reading light.
    rgb *= 1.0 - interior * 0.32;

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
