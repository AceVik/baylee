// Five original emblems, five earned silhouettes. No textures or copied artwork.
// Linear-light metal, recessed enamel, engraved bezels and gemstone settings.
// Motion only touches ornament, never text. Reduced motion has a designed still pose.
import bevy_ui_render::ui_vertex_output::UiVertexOutput;
import bevy_render::globals::Globals;
struct Params { metal: vec4<f32>, gem: vec4<f32>, shape: vec4<f32> }
@group(0) @binding(1) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> params: Params;
const PI: f32 = 3.14159265;
const TAU: f32 = 6.28318530;

fn stroke(d: f32, width: f32, aa: f32) -> f32 {
    return 1.0 - smoothstep(width - aa, width + aa, abs(d));
}
fn fill(d: f32, aa: f32) -> f32 { return 1.0 - smoothstep(-aa, aa, d); }
fn segment(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let q = p - a; let v = b - a;
    return length(q - v * clamp(dot(q, v) / dot(v, v), 0.0, 1.0));
}
fn diamond(p: vec2<f32>, size: vec2<f32>) -> f32 {
    return (abs(p.x) / size.x + abs(p.y) / size.y - 1.0) * min(size.x, size.y);
}

// Every emblem has a recognisable outer contour, even in its locked etching.
fn emblem(p: vec2<f32>, kind: f32, aa: f32) -> f32 {
    let r = length(p); let a = atan2(p.y, p.x);
    if kind < 0.5 { // Solar seal: solid sun with twelve separate rays.
        let sun = fill(r - 0.135, aa);
        let rays = stroke(sin(a * 6.0), 0.23, aa * 6.0) * fill(r - 0.255, aa) * fill(0.176 - r, aa);
        return max(sun, rays);
    }
    if kind < 1.5 { // Lunar observatory: crescent and a four-point star.
        let moon = min(fill(length(p + vec2<f32>(0.04, 0.0)) - 0.21, aa), 1.0 - fill(length(p - vec2<f32>(0.055, -0.045)) - 0.18, aa));
        let star = fill(diamond(p - vec2<f32>(0.15, -0.12), vec2<f32>(0.07, 0.105)), aa);
        return max(moon, star);
    }
    if kind < 2.5 { // Compass rose: four long points and an inset diamond.
        let arms = max(fill(diamond(p, vec2<f32>(0.065, 0.26)), aa), fill(diamond(p, vec2<f32>(0.22, 0.065)), aa));
        return max(arms, stroke(diamond(p, vec2<f32>(0.17)), 0.012, aa));
    }
    if kind < 3.5 { // True strike: an arrow passing through its target ring.
        let ring = stroke(length(p - vec2<f32>(0.02, -0.01)) - 0.16, 0.016, aa);
        let shaft = stroke(segment(p, vec2<f32>(-0.19, 0.22), vec2<f32>(0.14, -0.17)), 0.018, aa);
        let head = max(stroke(segment(p, vec2<f32>(0.14, -0.17), vec2<f32>(0.02, -0.16)), 0.018, aa), stroke(segment(p, vec2<f32>(0.14, -0.17), vec2<f32>(0.15, -0.05)), 0.018, aa));
        return max(ring, max(shaft, head));
    }
    // Living oak: layered canopy, trunk and roots; persistent progress, no streak reset.
    var tree = fill(length(p - vec2<f32>(0.0, -0.13)) - 0.13, aa);
    tree = max(tree, fill(length(p - vec2<f32>(0.12, -0.04)) - 0.105, aa));
    tree = max(tree, fill(length(p - vec2<f32>(-0.12, -0.04)) - 0.105, aa));
    tree = max(tree, stroke(segment(p, vec2<f32>(0.0, -0.04), vec2<f32>(0.0, 0.2)), 0.025, aa));
    tree = max(tree, stroke(segment(p, vec2<f32>(-0.13, 0.23), vec2<f32>(0.0, 0.15)), 0.014, aa));
    return max(tree, stroke(segment(p, vec2<f32>(0.13, 0.23), vec2<f32>(0.0, 0.15)), 0.014, aa));
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let p = (in.uv - 0.5) * 2.0;
    let r = length(p); let a = atan2(p.y, p.x);
    let aa = max(fwidth(r), 0.003);
    let tier = params.shape.y;
    let earned = select(0.0, 1.0, tier > 0.5);
    let t = globals.time * params.shape.z;
    let metal = params.metal.rgb;
    let gem = params.gem.rgb;
    let disc = fill(r - 0.53, aa);
    let edge = stroke(r - 0.51, 0.028, aa);
    let bevel = stroke(r - 0.465, 0.012, aa);
    let inset = fill(r - 0.44, aa);
    let engraving = stroke(r - 0.399, 0.005, aa);
    let light = clamp(0.56 - p.y * 0.42 - p.x * 0.24, 0.18, 1.0);
    var color = metal * light * 0.44;
    color = mix(color, metal * (0.55 + light), edge);
    color = mix(color, metal * 0.9, bevel);
    color = mix(color, mix(vec3<f32>(0.012, 0.018, 0.025), gem * 0.09, earned), inset);
    color = mix(color, metal * 0.38, engraving);
    let etch = emblem(p, params.shape.x, aa);
    color = mix(color, metal * (0.85 + light * 0.55), etch);
    var alpha = disc;
    // Silver: second engraved bezel and eight rivets.
    if tier > 1.5 {
        let outer = stroke(r - 0.585, 0.013, aa);
        let rivets = stroke(r - 0.56, 0.018, aa) * stroke(sin(a * 4.0), 0.13, aa * 3.0);
        let detail = max(outer, rivets);
        color = mix(color, metal * (0.65 + light), detail);
        alpha = max(alpha, detail);
    }
    // Gold: a mirrored laurel wreath, individual pointed leaves on curved stems.
    if tier > 2.5 {
        let stem = stroke(r - 0.70, 0.009, aa) * fill(abs(a + PI * 0.5) - 2.45, aa);
        var leaves = 0.0;
        for (var i = 0; i < 7; i = i + 1) {
            let angle = -1.05 + f32(i) * 0.34;
            let centre = vec2<f32>(cos(angle), sin(angle)) * 0.70;
            let q = vec2<f32>(abs(p.x), p.y) - centre;
            let leaf = vec2<f32>(q.x * cos(angle + 0.7) + q.y * sin(angle + 0.7), -q.x * sin(angle + 0.7) + q.y * cos(angle + 0.7));
            leaves = max(leaves, fill(diamond(leaf, vec2<f32>(0.092, 0.032)), aa));
        }
        let wreath = max(stem, leaves);
        color = mix(color, metal * (0.58 + light), wreath);
        alpha = max(alpha, wreath);
    }
    // Platinum: five gemstone sockets and a suspended pendant.
    if tier > 3.5 {
        for (var i = 0; i < 5; i = i + 1) {
            let angle = -PI * 0.5 + f32(i) * TAU / 5.0;
            let centre = vec2<f32>(cos(angle), sin(angle)) * 0.59;
            let cut = diamond(p - centre, vec2<f32>(0.043, 0.065));
            let socket = fill(cut - 0.014, aa);
            color = mix(color, metal, socket);
            color = mix(color, gem * (0.65 + light), fill(cut, aa));
            alpha = max(alpha, socket);
        }
        let pendant = fill(diamond(p - vec2<f32>(0.0, 0.79), vec2<f32>(0.085, 0.105)), aa);
        color = mix(color, gem * 0.9, pendant); alpha = max(alpha, pendant);
    }
    // Obsidian: a three-point crown, aurora halo and a restrained travelling glint.
    if tier > 4.5 {
        let q = p - vec2<f32>(0.0, -0.72);
        let crown = fill(max(abs(q.x) - 0.23, max(q.y - 0.09, -q.y - 0.055 - 0.12 * pow(abs(sin(q.x * 13.65)), 4.0))), aa);
        color = mix(color, metal * (0.65 + light), crown); alpha = max(alpha, crown);
    }
    let glint = pow(max(0.0, cos(p.x * 3.0 + p.y * 2.0 - t * 0.65)), 24.0) * 0.32 * earned * params.shape.z;
    color = color + metal * glint * max(edge, etch);
    let halo = exp(-pow((r - 0.77) * 10.0, 2.0)) * 0.10 * select(0.0, 1.0, tier > 4.5);
    let breath = 0.85 + 0.15 * sin(t * 0.7) * params.shape.z;
    color = mix(gem * 0.4 * breath, color, alpha);
    alpha = max(alpha, halo * breath);
    return vec4<f32>(color * mix(0.50, 1.0, earned), alpha);
}
