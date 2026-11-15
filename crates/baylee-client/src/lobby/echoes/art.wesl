// Original expedition objects, distinct from permanent achievement medals.
// Bounded analytic shapes, no texture fetches or CPU animation uploads.
import bevy_ui_render::ui_vertex_output::UiVertexOutput;
import bevy_render::globals::Globals;
struct Params { metal: vec4<f32>, gem: vec4<f32>, shape: vec4<f32> }
@group(0) @binding(1) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> params: Params;

fn fill(d: f32, aa: f32) -> f32 { return 1.0 - smoothstep(-aa, aa, d); }
fn line(d: f32, width: f32, aa: f32) -> f32 { return fill(abs(d) - width, aa); }
fn box(p: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    let d = abs(p) - size + radius;
    return length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0) - radius;
}
fn diamond(p: vec2<f32>, size: vec2<f32>) -> f32 {
    return (abs(p.x) / size.x + abs(p.y) / size.y - 1.0) * min(size.x, size.y);
}
fn segment(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let q = p - a; let v = b - a;
    return length(q - v * clamp(dot(q, v) / dot(v, v), 0.0, 1.0));
}

// x: outer shape, y: inset engraving. Every object has its own silhouette.
fn object(p: vec2<f32>, kind: f32, aa: f32) -> vec2<f32> {
    let r = length(p);
    if kind < 0.5 { // A bound spell folio, not borrowed card artwork.
        let body = fill(box(p, vec2<f32>(0.35, 0.43), 0.05), aa);
        let pages = line(box(p, vec2<f32>(0.28, 0.35), 0.02), 0.014, aa);
        let star = fill(diamond(p, vec2<f32>(0.15, 0.24)), aa);
        return vec2<f32>(body, max(pages, star));
    }
    if kind < 1.5 { // Heartwood talisman: two lobes tapering into a carved point.
        let q = vec2<f32>(abs(p.x), p.y);
        let lobe = length(q - vec2<f32>(0.19, -0.15)) - 0.22;
        let point = max(q.x * 0.90 + p.y * 0.68 - 0.28, -p.y - 0.15);
        let heart = fill(min(lobe, point), aa);
        let grain = line(sin((p.x + 0.12 * sin(p.y * 7.0)) * 34.0), 0.14, aa * 25.0);
        let inset = fill(diamond(p - vec2<f32>(0.0, -0.04), vec2<f32>(0.10, 0.19)), aa);
        return vec2<f32>(heart, max(grain * 0.24, inset) * heart);
    }
    if kind < 2.5 { // Traveler's satchel: shoulder loop, flap and clasp.
        let bag = fill(box(p - vec2<f32>(0.0, 0.08), vec2<f32>(0.35, 0.30), 0.08), aa);
        let strap = line(length((p + vec2<f32>(0.0, 0.12)) * vec2<f32>(1.0, 1.5)) - 0.28, 0.032, aa);
        let flap = line(box(p + vec2<f32>(0.0, 0.035), vec2<f32>(0.29, 0.16), 0.07), 0.018, aa);
        let clasp = fill(box(p - vec2<f32>(0.0, 0.10), vec2<f32>(0.065, 0.09), 0.018), aa);
        return vec2<f32>(max(bag, strap), max(flap, clasp));
    }
    if kind < 3.5 { // Compass: recessed face, north needle and cardinal ticks.
        let face = fill(r - 0.39, aa);
        let rim = line(r - 0.34, 0.018, aa);
        let needle = max(fill(diamond(p, vec2<f32>(0.075, 0.29)), aa), fill(diamond(p, vec2<f32>(0.25, 0.055)), aa));
        return vec2<f32>(face, max(rim, needle));
    }
    if kind < 4.5 { // Grove vessel: living canopy on a sculpted root pedestal.
        let crown = min(length(p + vec2<f32>(0.0, 0.22)) - 0.22, min(length(p + vec2<f32>(0.21, 0.07)) - 0.17, length(p - vec2<f32>(0.21, -0.07)) - 0.17));
        let trunk = line(segment(p, vec2<f32>(0.0, -0.03), vec2<f32>(0.0, 0.30)), 0.045, aa);
        let roots = line(segment(vec2<f32>(abs(p.x), p.y), vec2<f32>(0.0, 0.24), vec2<f32>(0.24, 0.36)), 0.03, aa);
        let canopy = fill(crown, aa);
        return vec2<f32>(max(canopy, max(trunk, roots)), canopy * (0.55 + 0.35 * sin(p.x * 12.0 + p.y * 9.0)));
    }
    // Three potion silhouettes share glass, each has a different internal mark.
    let body = fill(length((p - vec2<f32>(0.0, 0.13)) * vec2<f32>(1.0, 0.92)) - 0.30, aa);
    let neck = fill(box(p + vec2<f32>(0.0, 0.20), vec2<f32>(0.11, 0.20), 0.02), aa);
    let stopper = fill(box(p + vec2<f32>(0.0, 0.39), vec2<f32>(0.14, 0.05), 0.015), aa);
    let liquid = body * fill(0.04 - p.y, aa);
    var mark = fill(box(p - vec2<f32>(0.0, 0.14), vec2<f32>(0.035, 0.13), 0.01), aa);
    mark = max(mark, fill(box(p - vec2<f32>(0.0, 0.14), vec2<f32>(0.13, 0.035), 0.01), aa));
    if kind > 5.5 { mark = fill(diamond(p - vec2<f32>(0.0, 0.14), vec2<f32>(0.10, 0.17)), aa); }
    if kind > 6.5 { mark = line(diamond(p - vec2<f32>(0.0, 0.14), vec2<f32>(0.16, 0.20)), 0.025, aa); }
    return vec2<f32>(max(body, max(neck, stopper)), max(liquid * 0.45, mark));
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let p = (in.uv - 0.5) * 2.0;
    let r = length(p);
    let aa = max(fwidth(p.x), 0.003);
    let tier = params.shape.y;
    let t = globals.time * params.shape.z;
    let metal = params.metal.rgb;
    let gem = params.gem.rgb;
    let distance = box(p, vec2<f32>(0.86), 0.16);
    let base = fill(distance, aa);
    let rim = line(distance + 0.025, 0.018, aa);
    let light = clamp(0.8 - p.y * 0.4 - p.x * 0.3, 0.25, 1.3);
    var color = vec3<f32>(0.009, 0.016, 0.022) + gem * max(0.0, 0.12 - r * 0.10);
    color = mix(color, metal * light, rim);
    // Higher rarities add nested engraving, corner gems and an outer crown.
    if tier > 0.5 {
        let engraving = line(box(p, vec2<f32>(0.75), 0.11), 0.006, aa);
        color = mix(color, metal * 0.55, engraving);
    }
    if tier > 1.5 {
        let q = abs(p) - vec2<f32>(0.64);
        let setting = fill(diamond(q, vec2<f32>(0.09, 0.12)), aa);
        let stone = fill(diamond(q, vec2<f32>(0.047, 0.075)), aa);
        color = mix(color, metal * light, setting);
        color = mix(color, gem * select(0.6, 1.6, q.x + q.y < 0.0), stone);
    }
    let silhouette = object(p, params.shape.x, aa);
    let raised = object(p + vec2<f32>(0.014, 0.018), params.shape.x, aa).x;
    color = mix(color, vec3<f32>(0.002), max(0.0, raised - silhouette.x));
    let brushed = 0.93 + 0.07 * sin(p.y * 170.0);
    color = mix(color, metal * light * brushed, silhouette.x);
    color = mix(color, gem * light, silhouette.y);
    color += max(0.0, silhouette.x - raised) * metal * 0.65;
    if tier > 2.5 {
        let crown = line(diamond(p + vec2<f32>(0.0, 0.65), vec2<f32>(0.18, 0.10)), 0.023, aa);
        color = mix(color, metal * 1.2, crown);
        let halo = exp(-pow((r - 0.56) * 12.0, 2.0));
        color += gem * halo * 0.09 * (0.90 + 0.10 * sin(t * 0.65));
    }
    let glint = pow(max(0.0, cos(p.x * 3.1 + p.y * 1.9 - t * 0.55)), 30.0);
    color += metal * glint * max(rim, silhouette.x * 0.4) * (0.04 + tier * 0.08) * params.shape.z;
    return vec4<f32>(color, base);
}
