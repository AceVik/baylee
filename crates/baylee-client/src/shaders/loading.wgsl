// Original arithmetic artwork: a suspended brass astrolabe in a deep violet
// passage. One UI pass, no textures, particles/entities or offscreen buffers.
#import bevy_ui::ui_vertex_output::UiVertexOutput

struct PassageParams { scene: vec4<f32>, state: vec4<f32> }
@group(1) @binding(0) var<uniform> p: PassageParams;
const TAU: f32 = 6.28318530718;
fn hash(x: f32) -> f32 { return fract(sin(x * 127.1 + 31.7) * 43758.5453); }
fn line(d: f32, w: f32, aa: f32) -> f32 { return 1.0 - smoothstep(w, w + aa, abs(d)); }
fn ring(r: f32, radius: f32, aa: f32) -> f32 { return line(r - radius, 0.0007, aa); }

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let aspect = max(p.scene.x, 0.1);
    let flight = p.scene.z;
    let t = p.scene.y + flight * flight * 12.0 * (1.0 - p.scene.w);
    let still = p.scene.w;
    let uv = (in.uv - vec2<f32>(0.5, 0.44)) * vec2<f32>(aspect, 1.0);
    let screen_r = length(uv);
    let aa = max(fwidth(screen_r), 0.0004);
    let reveal = smoothstep(0.48, 1.0, flight) * length(vec2<f32>(aspect, 1.0));
    let alpha = select(select(1.0, smoothstep(reveal - 0.025, reveal + 0.018, screen_r), flight > 0.48), 1.0 - flight, still > 0.5);
    if (alpha < 0.002) { return vec4<f32>(0.0); }
    let zoom = 1.0 + pow(flight, 2.7) * 13.0 * (1.0 - still);
    let q = uv / zoom;
    let r = length(q);
    let a = atan2(q.y, q.x);
    let gold = vec3<f32>(0.72, 0.48, 0.21);
    let light = vec3<f32>(1.0, 0.81, 0.46);
    var c = vec3<f32>(0.008, 0.011, 0.026);
    // Broad, quiet nebular light; fine lines remain crisp above it.
    c += vec3<f32>(0.085, 0.034, 0.125) * exp(-r * r * 6.0);
    c += vec3<f32>(0.025, 0.07, 0.105) * exp(-length(q - vec2<f32>(0.32, -0.12)) * 5.0);
    c += gold * exp(-abs(r - 0.155) * 48.0) * 0.075;
    c += vec3<f32>(0.10, 0.052, 0.15) * (0.5 + 0.5 * sin(a * 3.0 + r * 25.0 - t * 0.08)) * exp(-r * 6.0) * 0.35;
    // Dark inset and a beveled brass carrier give the fine engraving depth.
    let carrier = 1.0 - smoothstep(0.0022, 0.0030, abs(r - 0.154));
    c *= 1.0 - (1.0 - smoothstep(0.141, 0.146, r)) * 0.36;
    let face_light = 0.45 + 0.55 * cos(a - 0.9);
    c += gold * carrier * (0.06 + max(0.0, face_light) * 0.13);
    c += light * ring(r, 0.1517, aa) * pow(max(0.0, cos(a + 0.8)), 8.0) * 0.36;
    let metal = 0.45 + 0.55 * pow(0.5 + 0.5 * cos(a - t * 0.12), 4.0);
    var etch = ring(r, 0.148, aa) * 0.4 + ring(r, 0.154, aa) * metal;
    etch += ring(r, 0.196, aa) * 0.22 + ring(r, 0.201, aa) * 0.09;
    // Engraved radial ticks, broken inner orbits and travelling highlights.
    let tick = abs(sin((a + 0.015 * t) * 48.0));
    etch += (1.0 - smoothstep(0.025, 0.09, tick)) * smoothstep(0.164, 0.166, r) * (1.0 - smoothstep(0.174, 0.176, r)) * 0.65;
    for (var i = 0; i < 3; i += 1) {
        let fi = f32(i);
        let angle = a + t * (0.045 + fi * 0.013) * select(1.0, -1.0, i == 1);
        let segmented = smoothstep(-0.7, -0.65, sin(angle * 3.0 + fi * 2.1));
        etch += ring(r, 0.100 + fi * 0.012, aa) * segmented * (0.22 + 0.1 * fi);
        let head = pow(max(0.0, cos(angle - fi * 2.1)), 60.0);
        c += light * exp(-abs(r - (0.100 + fi * 0.012)) * 750.0) * head * 0.7;
    }
    // Four mineral points, and a tiny breathing crystal at the aperture.
    let diamond = abs(q.x) + abs(q.y);
    etch += line(diamond - 0.071, 0.0007, aa) * 0.65;
    etch += line(diamond - 0.062, 0.0004, aa) * 0.23;
    let core = exp(-r * 110.0) * (0.7 + 0.12 * sin(t * 1.6));
    c += light * core * 1.3;
    // Four small cut stones hold the outer ring. The two diagonal faces
    // catch different light without adding geometry or another render pass.
    for (var i = 0; i < 4; i += 1) {
        let angle = f32(i) * TAU * 0.25 + TAU * 0.125;
        let stone = q - vec2<f32>(cos(angle), sin(angle)) * 0.196;
        let cut = abs(stone.x) + abs(stone.y);
        let shape = 1.0 - smoothstep(0.006, 0.006 + aa, cut);
        c += mix(gold * 0.18, light * 0.55, step(stone.x, stone.y)) * shape;
        c += light * line(cut - 0.006, 0.0004, aa) * 0.32;
    }
    // A faceted, transparent core catches the inward stream.
    let facet = (1.0 - smoothstep(0.058, 0.061, diamond)) * 0.08;
    c += mix(vec3<f32>(0.07, 0.15, 0.27), gold, step(q.x, -q.y)) * facet;
    c += gold * etch;
    // Each mote travels from the outer darkness into the aperture, then
    // respawns faintly at the edge. Tangential motion tightens as it falls.
    for (var i = 0; i < 28; i += 1) {
        let fi = f32(i);
        let life = fract(hash(fi) + t * (0.04 + hash(fi + 7.0) * 0.025) + flight * 0.68);
        let radial = 0.018 + (1.0 - life) * (1.0 - life) * (0.36 + hash(fi + 4.0) * 0.5);
        let angle = fi * 2.399963 + life * 1.25;
        let position = vec2<f32>(cos(angle), sin(angle)) * radial;
        let delta = q - position;
        let tangent = normalize(position + vec2<f32>(0.0001));
        let along = dot(delta, tangent);
        let across = dot(delta, vec2<f32>(-tangent.y, tangent.x));
        let tail = exp(-abs(along) / (0.002 + flight * 0.027)) * exp(-abs(across) / 0.0009);
        let envelope = smoothstep(0.0, 0.12, life) * (1.0 - smoothstep(0.83, 1.0, life));
        c += mix(gold, vec3<f32>(0.41, 0.52, 0.85), hash(fi + 11.0)) * tail * envelope * 0.9;
    }
    // Three lights correspond to actual preparation milestones, not time.
    for (var i = 0; i < 3; i += 1) {
        let fi = f32(i);
        let at = vec2<f32>((fi - 1.0) * 0.022, 0.235);
        let d = length(q - at);
        let lit = select(0.16, 1.0, p.state.x > fi);
        c += light * (exp(-d * 450.0) * 0.4 + (1.0 - smoothstep(0.0015, 0.0025, d))) * lit;
    }
    c += light * exp(-abs(screen_r - reveal) * 85.0) * smoothstep(0.48, 0.60, flight) * 0.45;
    // Dither only the original backdrop, never the table being revealed.
    c += (hash(dot(in.position.xy, vec2<f32>(0.067, 0.113))) - 0.5) / 255.0;
    return vec4<f32>(c, alpha);
}
