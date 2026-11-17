// Original expedition objects, distinct from permanent achievement medals.
// One shared image sample, bounded analytic settings, no CPU animation uploads.
struct Params {
    metal: vec4<f32>,
    gem: vec4<f32>,
    shape: vec4<f32>,
    atlas: vec4<f32>,
    clip: vec4<f32>,
}

fn fill(d: f32, aa: f32) -> f32 { return 1.0 - smoothstep(-aa, aa, d); }
fn line(d: f32, width: f32, aa: f32) -> f32 { return fill(abs(d) - width, aa); }
fn box(p: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    let d = abs(p) - size + radius;
    return length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0) - radius;
}
fn diamond(p: vec2<f32>, size: vec2<f32>) -> f32 {
    return (abs(p.x) / size.x + abs(p.y) / size.y - 1.0) * min(size.x, size.y);
}
fn draw(
    uv: vec2<f32>,
    params: Params,
    time: f32,
    portrait: texture_2d<f32>,
    portrait_sampler: sampler,
) -> vec4<f32> {
    let p = (uv - 0.5) * 2.0;
    let r = length(p);
    let aa = max(fwidth(p.x), 0.003);
    let motif = params.shape.x;
    let tier = params.shape.y;
    let energy = params.shape.z;
    let t = time * energy;
    let metal = params.metal.rgb;
    let gem = params.gem.rgb;
    let distance = box(p, vec2<f32>(0.86), 0.16);
    let base = fill(distance, aa);
    let rim = line(distance + 0.025, 0.018, aa);
    let light = clamp(0.8 - p.y * 0.4 - p.x * 0.3, 0.25, 1.3);

    // Subtle breathing aura behind the vessel, phased by motif so neighboring
    // items on a shelf breathe naturally rather than blinking in lockstep.
    let breath = 1.0 + 0.16 * sin(t * 1.15 + motif * 0.41) * energy;
    var color = vec3<f32>(0.009, 0.015, 0.022)
        + gem * max(0.0, 0.135 - r * 0.105) * (1.0 + tier * 0.14) * breath;
    color = mix(color, metal * light, rim);

    // Higher rarities add nested engraving, corner gems, arcane shimmer and a crown.
    var engraving = 0.0;
    if tier > 0.5 {
        engraving = line(box(p, vec2<f32>(0.75), 0.11), 0.006, aa);
        color = mix(color, metal * 0.58, engraving);
    }
    if tier > 1.5 {
        let q = abs(p) - vec2<f32>(0.64);
        let setting = fill(diamond(q, vec2<f32>(0.09, 0.12)), aa);
        let stone = fill(diamond(q, vec2<f32>(0.047, 0.075)), aa);
        let facet_twinkle = 1.0 + 0.22 * sin(t * 1.55 + (p.x - p.y) * 2.4 + motif * 0.3) * energy;
        color = mix(color, metal * light, setting);
        color = mix(color, gem * select(0.65, 1.65, q.x + q.y < 0.0) * facet_twinkle, stone);
    }

    var obj_alpha = 0.0;
    if motif < 0.5 {
        // Card offers keep our original folio, never borrowed card illustrations.
        let body = fill(box(p, vec2<f32>(0.35, 0.43), 0.05), aa);
        let pages = line(box(p, vec2<f32>(0.28, 0.35), 0.02), 0.014, aa);
        let star = fill(diamond(p, vec2<f32>(0.15, 0.24)), aa);
        obj_alpha = body;
        color = mix(color, metal * light, body);
        color = mix(color, gem * light, max(pages, star));
    } else {
        // Optically centered square window inside the inner engraving, clipped to
        // this item's exclusive cell so neighboring atlas cells can never bleed in.
        let image_uv = (uv - 0.5) / 0.68 + 0.5;
        let atlas_uv = image_uv * params.atlas.zw + params.atlas.xy;
        let clamped_uv = clamp(atlas_uv, params.clip.xy, params.clip.zw);
        let pixel = textureSample(portrait, portrait_sampler, clamped_uv);
        let within = all(image_uv >= vec2<f32>(0.0)) && all(image_uv <= vec2<f32>(1.0))
            && all(atlas_uv >= params.clip.xy) && all(atlas_uv <= params.clip.zw);
        obj_alpha = select(0.0, pixel.a, within);
        color = mix(color, pixel.rgb, obj_alpha);
    }

    let bg_mask = 1.0 - obj_alpha * 0.85;
    if tier > 1.5 {
        // Gentle arcane shimmer along the inner ring on Rare and Mythic items,
        // kept behind the object's opaque paint so item hue stays true.
        let angle = atan2(p.y, p.x);
        let ring = exp(-pow((r - 0.58) * 11.0, 2.0));
        let arc_wave = 0.55 + 0.45 * sin(angle * 3.0 - t * 0.95 + motif * 0.5) * energy;
        let shimmer_strength = select(0.055, 0.105, tier > 2.5);
        color += gem * ring * arc_wave * shimmer_strength * bg_mask;
    }
    if tier > 2.5 {
        let crown = line(diamond(p + vec2<f32>(0.0, 0.65), vec2<f32>(0.18, 0.10)), 0.023, aa);
        color = mix(color, metal * 1.25, crown);
        let halo = exp(-pow((r - 0.56) * 12.0, 2.0));
        color += gem * halo * 0.08 * (0.90 + 0.14 * sin(t * 0.85)) * bg_mask;
    }

    // Rarity-scaled metallic rim sheen: travelling specular catchlight + diagonal luster.
    let rim_angle = atan2(p.y, p.x);
    let sweep_sheen = pow(max(0.0, cos(rim_angle - t * 0.52 + motif * 0.23)), 16.0);
    let diag_glint = pow(max(0.0, cos(p.x * 3.1 + p.y * 1.9 - t * 0.62)), 28.0);
    let metal_mask = max(rim, engraving * 0.65);
    color += metal * (diag_glint * (0.05 + tier * 0.075) + sweep_sheen * (0.03 + tier * 0.055)) * metal_mask * energy;

    // Passing states and shop emphasis bits, set only when state changes.
    let veil = u32(params.shape.w);
    let owned = (veil & 8u) != 0u;
    let fusion = (veil & 112u) != 0u;
    let frozen = (veil & 128u) != 0u;

    // Shop affinity never modifies the portrait. Matching copies illuminate
    // the frame; a completing purchase adds a stronger tiered orbit and gems.
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

    // Frozen shop state: crystalline glacier-blue rime frost creeping inward from
    // the edges and corners + slow frost-crystal sparkle.
    if frozen {
        let edge_in = max(-distance, 0.0);
        let facets = abs(sin(p.x * 18.0 + sin(p.y * 12.0) * 1.35))
            * abs(cos(p.y * 16.0 - sin(p.x * 10.5) * 1.35));
        let corner_frost = smoothstep(0.32, 0.88, abs(p.x) * abs(p.y) * 1.9);
        let reach = 0.16 + 0.085 * facets + 0.10 * corner_frost;
        let rime = smoothstep(reach, 0.01, edge_in) * base;
        let ice_vein = line(edge_in - 0.055 - 0.032 * facets, 0.0075, aa) * base;

        let grid = (p + 1.0) * 8.5;
        let cell = floor(grid);
        let h = fract(sin(dot(cell, vec2<f32>(12.9898, 78.233))) * 43758.5453);
        let local_d = length(fract(grid) - 0.5);
        let twinkle = pow(max(0.0, sin(t * (0.75 + h * 0.65) + h * 6.2831)), 14.0) * energy;
        let sparkle = smoothstep(0.17, 0.0, local_d) * step(0.58, h)
            * twinkle * smoothstep(0.34, 0.015, edge_in) * base;

        let ice_deep = vec3<f32>(0.07, 0.32, 0.66);
        let ice_bright = vec3<f32>(0.50, 0.84, 1.0);
        color = mix(color, color * vec3<f32>(0.84, 0.94, 1.06) + ice_deep * 0.045, 0.38 * base * bg_mask);
        color = mix(color, mix(ice_deep, ice_bright, 0.42 + 0.48 * facets), rime * 0.82);
        color += ice_bright * (ice_vein * 0.38 + sparkle * 0.85);
    }

    if (veil & 2u) != 0u {
        color = mix(color, vec3<f32>(dot(color, vec3<f32>(0.2126, 0.7152, 0.0722))), 0.85) * 0.55;
    }
    if (veil & 1u) != 0u { color *= 0.40; }
    if (veil & 4u) != 0u { color = mix(color, vec3<f32>(0.12, 0.18, 0.30), 0.25); }
    return vec4<f32>(color, base);
}
