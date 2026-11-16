// Original expedition objects, distinct from permanent achievement medals.
// One shared image sample, bounded analytic settings, no CPU animation uploads.
struct Params { metal: vec4<f32>, gem: vec4<f32>, shape: vec4<f32>, atlas: vec4<f32> }

fn fill(d: f32, aa: f32) -> f32 { return 1.0 - smoothstep(-aa, aa, d); }
fn line(d: f32, width: f32, aa: f32) -> f32 { return fill(abs(d) - width, aa); }
fn box(p: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    let d = abs(p) - size + radius;
    return length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0) - radius;
}
fn diamond(p: vec2<f32>, size: vec2<f32>) -> f32 {
    return (abs(p.x) / size.x + abs(p.y) / size.y - 1.0) * min(size.x, size.y);
}
fn draw(uv: vec2<f32>, params: Params, time: f32, portrait: texture_2d<f32>, portrait_sampler: sampler) -> vec4<f32> {
    let p = (uv - 0.5) * 2.0;
    let r = length(p);
    let aa = max(fwidth(p.x), 0.003);
    let tier = params.shape.y;
    let t = time * params.shape.z;
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
    if params.shape.x < 0.5 {
        // Card offers keep our original folio, never borrowed card illustrations.
        let body = fill(box(p, vec2<f32>(0.35, 0.43), 0.05), aa);
        let pages = line(box(p, vec2<f32>(0.28, 0.35), 0.02), 0.014, aa);
        let star = fill(diamond(p, vec2<f32>(0.15, 0.24)), aa);
        color = mix(color, metal * light, body);
        color = mix(color, gem * light, max(pages, star));
    } else {
        // Keep the complete transparent image inside the rarity setting. The
        // alpha bound prevents clamp-to-edge streaks from faint edge glows.
        let image_uv = (uv - 0.5) / 0.68 + 0.5;
        let pixel = textureSample(portrait, portrait_sampler, image_uv * params.atlas.zw + params.atlas.xy);
        let within = all(image_uv >= vec2<f32>(0.0)) && all(image_uv <= vec2<f32>(1.0));
        color = mix(color, pixel.rgb, select(0.0, pixel.a, within));
    }
    if tier > 2.5 {
        let crown = line(diamond(p + vec2<f32>(0.0, 0.65), vec2<f32>(0.18, 0.10)), 0.023, aa);
        color = mix(color, metal * 1.2, crown);
        let halo = exp(-pow((r - 0.56) * 12.0, 2.0));
        color += gem * halo * 0.09 * (0.90 + 0.10 * sin(t * 0.65));
    }
    let glint = pow(max(0.0, cos(p.x * 3.1 + p.y * 1.9 - t * 0.55)), 30.0);
    color += metal * glint * rim * (0.04 + tier * 0.08) * params.shape.z;
    // Table-only passing states (cardmat::veil), set when the state changes.
    let veil = u32(params.shape.w);
    // Shop affinity never modifies the portrait. Matching copies illuminate
    // the frame; a completing purchase adds a stronger tiered orbit and gems.
    let owned = (veil & 8u) != 0u;
    let fusion = (veil & 112u) != 0u;
    if owned || fusion {
        let strength = select(0.55, 1.10, fusion);
        let aura = exp(-abs(distance + 0.02) * 45.0);
        let pulse = 0.90 + 0.10 * sin(t * 1.4);
        var upgrade = vec3<f32>(0.42, 0.80, 0.85);
        if (veil & 16u) != 0u { upgrade = vec3<f32>(0.80, 0.91, 1.0); }
        if (veil & 32u) != 0u { upgrade = vec3<f32>(1.0, 0.62, 0.16); }
        if (veil & 64u) != 0u { upgrade = vec3<f32>(1.0, 0.28, 0.08); }
        color += upgrade * aura * strength * pulse;
        if fusion {
            let sweep = pow(max(0.0, cos(atan2(p.y, p.x) * 2.0 - t * 1.2)), 10.0);
            color += upgrade * rim * sweep * 1.4;
            let flourish = line(diamond(abs(p) - vec2<f32>(0.67), vec2<f32>(0.13, 0.08)), 0.016, aa);
            color = mix(color, upgrade * (0.9 + 0.3 * sweep), flourish);
        }
    }
    if (veil & 2u) != 0u {
        color = mix(color, vec3<f32>(dot(color, vec3<f32>(0.2126, 0.7152, 0.0722))), 0.85) * 0.55;
    }
    if (veil & 1u) != 0u { color *= 0.40; }
    if (veil & 4u) != 0u { color = mix(color, vec3<f32>(0.12, 0.18, 0.30), 0.25); }
    return vec4<f32>(color, base);
}
