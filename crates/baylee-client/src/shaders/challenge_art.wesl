// Original landscape miniatures: sunlit ruins, a lunar observatory and a woodland tower.
// Static decoration: no clock, texture, network asset or gameplay information.
import bevy_ui_render::ui_vertex_output::UiVertexOutput;
@group(1) @binding(0) var<uniform> scene: vec4<f32>;

fn fill(distance: f32, aa: f32) -> f32 {
    return 1.0 - smoothstep(-aa, aa, distance);
}
fn box(p: vec2<f32>, half: vec2<f32>) -> f32 {
    let q = abs(p) - half;
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0);
}
fn line(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let d = b - a;
    return length(p - a - d * clamp(dot(p - a, d) / dot(d, d), 0.0, 1.0));
}
fn pine(p: vec2<f32>, x: f32, base: f32, height: f32, aa: f32) -> f32 {
    let y = (p.y - (base - height)) / height;
    let width = height * (0.13 + y * 0.30);
    let crown = max(abs(p.x - x) - width, max(-y * height, (y - 0.88) * height));
    let trunk = box(p - vec2<f32>(x, base - height * 0.08), vec2<f32>(height * 0.045, height * 0.15));
    return fill(min(crown, trunk), aa);
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let p = vec2<f32>(in.uv.x * 2.0, in.uv.y);
    let aa = max(fwidth(p.x), 0.002);
    let night = scene.x > 0.5 && scene.x < 1.5;
    let tower = scene.x > 1.5;
    let top = select(vec3<f32>(0.018, 0.055, 0.067), vec3<f32>(0.022, 0.027, 0.095), night);
    let horizon = select(vec3<f32>(0.28, 0.13, 0.045), vec3<f32>(0.07, 0.10, 0.23), night);
    var color = mix(top, horizon, pow(in.uv.y, 1.6));
    let orb = p - vec2<f32>(1.43, 0.29);
    let halo = exp(-dot(orb, orb) * 14.0);
    let glow = select(vec3<f32>(0.75, 0.33, 0.075), vec3<f32>(0.24, 0.40, 0.70), night);
    color += glow * halo * 0.18;
    var disc = fill(length(orb) - 0.115, aa);
    if night {
        disc *= 1.0 - fill(length(orb - vec2<f32>(0.055, -0.035)) - 0.107, aa);
        for (var i = 0; i < 14; i += 1) {
            let x = 0.13 + fract(f32(i) * 0.618) * 1.72;
            let y = 0.09 + fract(f32(i) * 0.381) * 0.37;
            let star = p - vec2<f32>(x, y);
            let shape = min(box(star, vec2<f32>(0.010, 0.002)), box(star, vec2<f32>(0.002, 0.010)));
            color += vec3<f32>(0.32, 0.48, 0.63) * fill(shape, aa);
        }
    }
    color = mix(color, glow, disc);
    // Overlapping ridgelines create depth without a noisy texture.
    for (var layer = 0; layer < 3; layer += 1) {
        let f = f32(layer);
        let ridge = 0.52 + f * 0.105 + sin(p.x * (3.2 + f) + f * 2.1) * 0.075
            + sin(p.x * 7.0 + f) * 0.025;
        let ink = mix(top * 1.9, vec3<f32>(0.008, 0.029, 0.032), f * 0.35);
        color = mix(color, ink, smoothstep(ridge - aa, ridge + aa, p.y));
    }
    // The day path leads between weathered columns; the night path to an orrery.
    let stone = select(vec3<f32>(0.14, 0.14, 0.10), vec3<f32>(0.09, 0.15, 0.22), night);
    if !tower {
        for (var i = 0; i < 3; i += 1) {
            let x = 0.42 + f32(i) * 0.25;
            let height = select(0.26, 0.34, i == 1);
            let shaft = box(p - vec2<f32>(x, 0.69 - height * 0.5), vec2<f32>(0.040, height * 0.5));
            let cap = box(p - vec2<f32>(x, 0.69 - height), vec2<f32>(0.061, 0.025));
            let base = box(p - vec2<f32>(x, 0.70), vec2<f32>(0.067, 0.024));
            color = mix(color, stone, fill(min(min(shaft, cap), base), aa));
            color = mix(color, stone * 1.7, fill(box(p - vec2<f32>(x - 0.024, 0.69 - height * 0.5), vec2<f32>(0.007, height * 0.45)), aa));
        }
        if night {
            let q = p - vec2<f32>(0.67, 0.39);
            let ring = abs(length(q * vec2<f32>(1.0, 0.65)) - 0.17) - 0.006;
            color = mix(color, vec3<f32>(0.37, 0.48, 0.57), fill(ring, aa));
            color = mix(color, glow, fill(length(q) - 0.024, aa));
        }
    } else {
        let wall = box(p - vec2<f32>(0.94, 0.47), vec2<f32>(0.105, 0.26));
        color = mix(color, stone * 0.7, fill(wall, aa));
        color = mix(color, stone * 1.3, fill(box(p - vec2<f32>(0.88, 0.47), vec2<f32>(0.023, 0.26)), aa));
        let roof = max(abs(p.x - 0.94) - (p.y - 0.08) * 0.65, max(0.08 - p.y, p.y - 0.28));
        color = mix(color, vec3<f32>(0.035, 0.11, 0.095), fill(roof, aa));
        for (var floor = 0; floor < 4; floor += 1) {
            let window = p - vec2<f32>(0.95, 0.32 + f32(floor) * 0.09);
            color += glow * exp(-dot(window, window) * 2200.0) * 0.12;
            color = mix(color, glow * 0.7, fill(box(window, vec2<f32>(0.015, 0.024)), aa));
        }
        let bridge = line(p, vec2<f32>(0.60, 0.76), vec2<f32>(0.93, 0.69));
        color = mix(color, stone, fill(bridge - 0.018, aa));
    }
    for (var i = 0; i < 9; i += 1) {
        let x = f32(i) * 0.24 + 0.03;
        let height = 0.12 + fract(f32(i) * 0.41) * 0.22;
        let trees = pine(p, x, 0.88 + sin(x * 5.0) * 0.03, height, aa);
        color = mix(color, vec3<f32>(0.007, 0.024, 0.023), trees);
    }
    let path = abs(p.x - (0.83 + (p.y - 0.75) * 0.85));
    let near_path = fill(path - max(0.0, p.y - 0.72) * 0.20, aa) * smoothstep(0.73, 0.85, p.y);
    color = mix(color, stone * 0.75, near_path);
    let edge = box(p - vec2<f32>(1.0, 0.5), vec2<f32>(0.945, 0.445)) - 0.05;
    let trim = 1.0 - smoothstep(0.001, 0.008, abs(edge + 0.018));
    color = mix(color, glow * 0.38, trim * 0.8);
    return vec4<f32>(color, fill(edge, aa));
}
