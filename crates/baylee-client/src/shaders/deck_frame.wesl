// Animated achievement & favourite frame for deck tiles in the Lobby shelf.
//
// Drawn on an overlay quad (`Pickable::IGNORE`) covering the deck tile.
// The interior is completely transparent so the card artwork (`art_crop`)
// and text underneath are never covered or tinted; only the perimeter border
// and corner ornaments are rendered.

import bevy_render::globals::Globals;
import bevy_ui_render::ui_vertex_output::UiVertexOutput;

struct DeckFrameParams {
    tier: u32,
    favourite: u32,
    motion: f32,
    hover: f32,
}

@group(0) @binding(1) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> params: DeckFrameParams;

fn sd_rounded_box(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - b + vec2<f32>(r);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0))) - r;
}

fn hash21(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let fw = max(fwidth(uv), vec2<f32>(0.0005, 0.0005));

    let tier = params.tier;
    let fav = params.favourite != 0u;
    let hov = clamp(params.hover, 0.0, 1.0);
    if tier == 0u && !fav && hov < 0.01 {
        return vec4<f32>(0.0);
    }

    let size = 1.0 / fw;
    let half = size * 0.5;
    let p = (uv - 0.5) * size;

    let radius = 10.0;
    let dist_outer = sd_rounded_box(p, half - vec2<f32>(0.5), radius);
    if dist_outer > 1.0 {
        return vec4<f32>(0.0);
    }

    // Border thickness grows slightly with rank tier (1.8px .. 3.6px).
    let base_thick = select(
        1.6,
        1.8 + f32(min(tier, 11u)) * 0.16,
        tier > 0u,
    ) + hov * 0.45;
    let outer_aa = 1.0 - smoothstep(-0.6, 0.6, dist_outer);
    let inner_edge = -base_thick;
    let ring = outer_aa * smoothstep(inner_edge - 1.1, inner_edge + 0.3, dist_outer);

    // Subtle secondary inner filigree line for Tier >= 6.
    var inner_filigree = 0.0;
    if tier >= 6u {
        let fil_dist = abs(dist_outer + base_thick + 2.4);
        inner_filigree = outer_aa * (1.0 - smoothstep(0.0, 0.95, fil_dist)) * 0.55;
    }

    // Corner ornament geometry (4 corners) for Tier >= 2.
    let corner_dist = length(max(abs(p) - (half - vec2<f32>(18.0)), vec2<f32>(0.0)));
    let near_corner = smoothstep(16.0, 2.0, corner_dist);
    let diag = abs(abs(p.x) - half.x - (abs(p.y) - half.y));
    var corner_ornament = 0.0;
    if tier >= 2u {
        let orn_w = 1.2 + f32(min(tier, 11u)) * 0.12;
        let orn_band = 1.0 - smoothstep(0.0, orn_w, abs(dist_outer + base_thick + 1.6));
        corner_ornament = near_corner * orn_band * (0.35 + f32(min(tier, 11u)) * 0.055);
        if tier >= 5u {
            let diamond = (1.0 - smoothstep(0.0, 2.5, diag)) * near_corner * smoothstep(-base_thick - 5.5, -1.0, dist_outer);
            corner_ornament = max(corner_ornament, diamond * 0.75);
        }
    }

    // Soft inward glow on high tiers or hover.
    let glow_reach = 5.0 + f32(min(tier, 11u)) * 0.9 + hov * 5.0;
    let inner_glow = outer_aa
        * smoothstep(-glow_reach, 0.0, dist_outer)
        * (0.06 + f32(min(tier, 11u)) * 0.014 + hov * 0.10);

    let total_mask = clamp(ring + inner_filigree + corner_ornament + inner_glow, 0.0, 1.0);
    if total_mask <= 0.002 {
        return vec4<f32>(0.0);
    }

    let t = globals.time * params.motion * (1.0 + hov * 0.45);
    let angle = atan2(p.y, p.x);

    // Tier base & highlight palettes (Stufe 0..=11).
    var col_a = vec3<f32>(0.42, 0.48, 0.58);
    var col_b = vec3<f32>(0.68, 0.75, 0.86);
    var col_spark = vec3<f32>(1.0, 0.96, 0.84);

    switch tier {
        case 1u: {
            // Stufe 1: Kupfer
            col_a = vec3<f32>(0.64, 0.32, 0.22);
            col_b = vec3<f32>(0.92, 0.55, 0.38);
        }
        case 2u: {
            // Stufe 2: Eisen
            col_a = vec3<f32>(0.38, 0.43, 0.50);
            col_b = vec3<f32>(0.74, 0.80, 0.88);
        }
        case 3u: {
            // Stufe 3: Bronze
            col_a = vec3<f32>(0.55, 0.36, 0.20);
            col_b = vec3<f32>(0.83, 0.58, 0.36);
        }
        case 4u: {
            // Stufe 4: Silber
            col_a = vec3<f32>(0.58, 0.65, 0.74);
            col_b = vec3<f32>(0.93, 0.96, 1.00);
        }
        case 5u: {
            // Stufe 5: Smaragd
            col_a = vec3<f32>(0.10, 0.56, 0.38);
            col_b = vec3<f32>(0.38, 0.95, 0.68);
            col_spark = vec3<f32>(0.75, 1.00, 0.88);
        }
        case 6u: {
            // Stufe 6: Saphir
            col_a = vec3<f32>(0.14, 0.40, 0.82);
            col_b = vec3<f32>(0.45, 0.82, 1.00);
            col_spark = vec3<f32>(0.82, 0.95, 1.00);
        }
        case 7u: {
            // Stufe 7: Gold
            col_a = vec3<f32>(0.76, 0.56, 0.14);
            col_b = vec3<f32>(1.00, 0.86, 0.38);
        }
        case 8u: {
            // Stufe 8: Rubin
            col_a = vec3<f32>(0.78, 0.16, 0.24);
            col_b = vec3<f32>(1.00, 0.46, 0.50);
            col_spark = vec3<f32>(1.00, 0.86, 0.86);
        }
        case 9u: {
            // Stufe 9: Amethyst
            col_a = vec3<f32>(0.46, 0.22, 0.78);
            col_b = vec3<f32>(0.82, 0.58, 1.00);
            col_spark = vec3<f32>(0.94, 0.86, 1.00);
        }
        case 10u: {
            // Stufe 10: Prismatischer Diamant
            let prism = 0.5 + 0.5 * cos(angle * 2.0 + t * 1.4 + vec3<f32>(0.0, 2.1, 4.2));
            col_a = mix(vec3<f32>(0.62, 0.75, 0.95), prism, 0.55);
            col_b = mix(vec3<f32>(0.96, 0.98, 1.00), prism, 0.35);
            col_spark = vec3<f32>(1.0, 1.0, 1.0);
        }
        case 11u: {
            // Stufe 11: Ewiger Äther-Kronrahmen
            let aurora = 0.5 + 0.5 * cos(angle * 3.0 - t * 1.8 + vec3<f32>(0.4, 1.9, 3.8));
            col_a = mix(vec3<f32>(0.85, 0.62, 0.18), vec3<f32>(0.62, 0.28, 0.92), aurora.x);
            col_b = mix(vec3<f32>(1.00, 0.90, 0.48), vec3<f32>(0.55, 0.92, 1.00), aurora.y);
            col_spark = vec3<f32>(1.0, 0.98, 0.90);
        }
        default: {}
    }

    // Travelling metallic / arcane light wave around the perimeter.
    let speed = 0.8 + f32(min(tier, 11u)) * 0.14;
    let wave1 = 0.5 + 0.5 * sin(angle * 2.0 - t * speed);
    let wave2 = 0.5 + 0.5 * cos(angle * 3.0 + t * speed * 0.72);
    let sheen = pow(wave1, 2.2) * 0.65 + pow(wave2, 3.0) * 0.35;

    var color = mix(col_a, col_b, sheen);

    // Higher tiers get a sharp specular glint and starlight sparkles.
    if tier >= 4u {
        let glint = pow(max(0.0, cos(angle - t * speed * 1.15)), 10.0);
        color = mix(color, col_spark, glint * (0.35 + f32(min(tier, 11u)) * 0.04));
    }
    if tier >= 8u {
        let cell = floor((p + half) / 6.0);
        let n = hash21(cell);
        let twinkle = pow(0.5 + 0.5 * sin(t * 3.2 + n * 6.2831), 12.0);
        let on_rim = ring * step(0.78, n);
        color += col_spark * twinkle * on_rim * 0.55;
    }

    // Favourite decks ("leicht goldig umrandet") blend a warm golden rim.
    if fav {
        let gold_a = vec3<f32>(0.82, 0.64, 0.20);
        let gold_b = vec3<f32>(1.00, 0.88, 0.46);
        let gold_sheen = 0.5 + 0.5 * sin(angle * 2.0 - t * 1.1);
        let gold = mix(gold_a, gold_b, gold_sheen);
        let fav_weight = select(0.42, 0.92, tier == 0u);
        color = mix(color, gold, fav_weight);
    }

    // Hover brightens the rim smoothly.
    color = mix(color, col_spark, hov * 0.22);

    let base_alpha = select(
        clamp(f32(fav) * 0.78 + hov * 0.55, 0.0, 1.0),
        0.84 + f32(min(tier, 11u)) * 0.014,
        tier > 0u,
    );
    return vec4<f32>(color, total_mask * base_alpha);
}
