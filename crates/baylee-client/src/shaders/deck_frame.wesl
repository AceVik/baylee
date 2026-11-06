// Animated achievement & favourite frame for deck tiles in the Lobby shelf.
//
// Drawn on an overlay quad (`Pickable::IGNORE`) over the deck tile, reaching
// `params.halo` past its edge. The inside of the tile is completely
// transparent, so the card artwork (`art_crop`) and text are never covered or
// tinted: everything here lies within `params.room` of the tile's edge.
//
// The rarer the tier, the richer the frame, and each tier has a look of its
// own: hammered iron, engraved bronze, polished silver, cut emerald, a
// standing wave in sapphire, gold guilloche, beating ruby beads, amethyst
// runes, a mythic arc and an aurora. Ornament is mirrored about both axes.
// Light moves slowly and nothing strobes; with `motion` at zero the frame
// stands still and keeps its ornament.

import bevy_render::globals::Globals;
import bevy_ui_render::ui_vertex_output::UiVertexOutput;

struct DeckFrameParams {
    tier: u32,
    favourite: u32,
    motion: f32,
    hover: f32,
    // Physical pixels per logical one.
    scale: f32,
    // How far the quad reaches past the tile, logical pixels.
    halo: f32,
    // How deep the frame may reach into the tile, logical pixels.
    room: f32,
}

@group(0) @binding(1) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> params: DeckFrameParams;

const TAU: f32 = 6.2831853;
const PI: f32 = 3.1415927;
// `tokens::RADIUS_PANEL`, the tile's own corner.
const RADIUS: f32 = 14.0;

fn sd_rounded_box(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - b + vec2<f32>(r);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0))) - r;
}

fn hash21(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

// Arc length round the box, clockwise from the top left. Continuous along the
// rim, which is where it is read.
fn rim_length(p: vec2<f32>, half: vec2<f32>) -> f32 {
    let gap = half - abs(p);
    if gap.y < gap.x {
        if p.y < 0.0 {
            return p.x + half.x;
        }
        return 3.0 * half.x + 2.0 * half.y - p.x;
    }
    if p.x > 0.0 {
        return 2.0 * half.x + p.y + half.y;
    }
    return 4.0 * half.x + 3.0 * half.y - p.y;
}

// A band of width `w` at depth `at`.
fn line(e: f32, at: f32, w: f32, aa: f32) -> f32 {
    return 1.0 - smoothstep(w * 0.5 - aa, w * 0.5 + aa, abs(e - at));
}

// `rgb` at opacity `a` laid over `base`, straight alpha.
fn over(base: vec4<f32>, rgb: vec3<f32>, a: f32) -> vec4<f32> {
    let out_a = a + base.a * (1.0 - a);
    let mixed = rgb * a + base.rgb * base.a * (1.0 - a);
    return vec4<f32>(mixed / max(out_a, 0.0001), out_a);
}

// Light gathered on a head that laps the rim; `at` and `head` run 0..1.
fn comet(at: f32, head: f32, tail: f32) -> f32 {
    let behind = fract(head - at);
    return exp(-behind / tail) * smoothstep(0.0, tail * 0.2, behind);
}

// A four-pointed gleam at `v`, its rays along the tile's edges.
fn gleam(v: vec2<f32>, size: f32) -> f32 {
    let core = exp(-dot(v, v) / (size * size * 0.08));
    let ax = exp(-abs(v.y) * 5.0 / size) * exp(-abs(v.x) * 1.1 / size);
    let ay = exp(-abs(v.x) * 5.0 / size) * exp(-abs(v.y) * 1.1 / size);
    return core + (ax + ay) * 0.55;
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let fw = max(fwidth(uv), vec2<f32>(0.0002, 0.0002));

    let tier = min(params.tier, 11u);
    let tf = f32(tier);
    let fav = params.favourite != 0u;
    let hov = clamp(params.hover, 0.0, 1.0);
    if tier == 0u && !fav && hov < 0.01 {
        return vec4<f32>(0.0);
    }

    let unit = max(params.scale, 0.5);
    let aa = 0.7 / unit;
    let size = 1.0 / fw / unit;
    let half = size * 0.5 - vec2<f32>(params.halo);
    let p = (uv - 0.5) * size;
    // Depth into the tile: positive within it, negative on the halo.
    let e = -sd_rounded_box(p, half, RADIUS);
    let room = params.room;
    if e > room + 1.0 {
        return vec4<f32>(0.0);
    }

    let t = globals.time * params.motion;
    let gap = half - abs(p);
    let on_row = gap.y < gap.x;
    let rim_len = 4.0 * (half.x + half.y);
    // 0..1 round the rim.
    let ph = rim_length(p, half) / rim_len;

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
            // Stufe 10: Prismatischer Funke
            let prism = 0.5 + 0.5 * cos(TAU * (ph * 2.0 - t * 0.09) + vec3<f32>(0.0, 2.1, 4.2));
            col_a = mix(vec3<f32>(0.62, 0.75, 0.95), prism, 0.55);
            col_b = mix(vec3<f32>(0.96, 0.98, 1.00), prism, 0.35);
            col_spark = vec3<f32>(1.0, 1.0, 1.0);
        }
        case 11u: {
            // Stufe 11: Ewiger Äther-Kronrahmen
            let aurora = 0.5 + 0.5 * cos(TAU * (ph * 3.0 + t * 0.07) + vec3<f32>(0.4, 1.9, 3.8));
            col_a = mix(vec3<f32>(0.85, 0.62, 0.18), vec3<f32>(0.62, 0.28, 0.92), aurora.x);
            col_b = mix(vec3<f32>(1.00, 0.90, 0.48), vec3<f32>(0.55, 0.92, 1.00), aurora.y);
            col_spark = vec3<f32>(1.0, 0.98, 0.90);
        }
        default: {}
    }

    // The rim: a rounded bar of metal, heavier with every tier.
    let rim_w = select(1.6, 1.4 + tf * 0.2, tier > 0u) + hov * 0.4;
    let inside = smoothstep(-aa, aa, e);
    let rim_mask = inside * (1.0 - smoothstep(rim_w - aa, rim_w + aa, e));
    let crown = 1.0 - abs(clamp(e / max(rim_w, 0.5), 0.0, 1.0) * 2.0 - 1.0);
    let lap = 0.045 + tf * 0.008;
    let sheen = pow(0.5 + 0.5 * cos(TAU * (ph * 2.0 - t * lap)), 3.0);
    var metal = mix(col_a, col_b, clamp(0.18 + crown * 0.5 + sheen * 0.55, 0.0, 1.0));

    // Beaten metal shows the hammer; a cut stone shows its facets.
    if tier >= 1u && tier <= 3u {
        let dent = hash21(vec2<f32>(floor(ph * rim_len / 3.0), 1.0));
        let depth = select(0.10, 0.26, tier == 2u);
        metal *= 1.0 - depth * 0.5 + dent * depth;
    }

    // What is mirrored stands on `u`: a whole number at each edge's middle and
    // its corners, so that nothing is cut off where two edges meet.
    let along = select(abs(p.y), abs(p.x), on_row);
    let span = select(half.y, half.x, on_row);
    let period = select(9.0, 7.0, tier >= 8u);
    let cells = max(round(span / period), 1.0);
    let u = along / (span / cells);
    let cell = floor(u);
    let in_cell = fract(u);

    if tier == 5u {
        let facet = 1.0 - abs(in_cell * 2.0 - 1.0);
        let fire = pow(0.5 + 0.5 * sin(cell * 2.4 + t * 0.9), 6.0);
        metal = mix(metal * (0.72 + facet * 0.5), col_spark, fire * 0.7);
    }

    var out = vec4<f32>(metal, 0.0);
    var strength = 0.0;

    // A soft light falling inward from the rim.
    let fade = 1.0 - smoothstep(room - 2.5, room, e);
    let glow_at = inside * fade * exp(-max(e - rim_w, 0.0) / (2.2 + tf * 0.3));
    out = over(out, col_b, glow_at * (0.05 + tf * 0.016 + hov * 0.12));

    // The heart of the ruby, and of everything above it: a slow double beat.
    let beat_at = t * 1.9;
    let beat = pow(max(0.0, sin(beat_at)), 10.0) + 0.6 * pow(max(0.0, sin(beat_at - 0.7)), 10.0);

    // Past the tile's edge, from gold up: a halo that breathes.
    if tier >= 7u && params.halo > 0.0 && e < 0.0 {
        let breath = 0.75 + 0.25 * sin(t * 0.8);
        var lift = breath;
        if tier == 8u {
            lift += beat * 0.5;
        }
        let spill = exp(e / (params.halo * 0.42)) * (1.0 - smoothstep(params.halo * 0.8, params.halo, -e));
        let halo_rgb = mix(col_a, col_b, 0.5 + 0.5 * sheen);
        return vec4<f32>(halo_rgb, spill * (0.12 + (tf - 7.0) * 0.035 + hov * 0.1) * lift);
    }
    if e < -1.0 {
        return vec4<f32>(0.0);
    }

    // The ornament band, between the rim and the room left.
    let band_in = rim_w + 1.0;
    let band_out = room - 0.5;
    let band_mid = (band_in + band_out) * 0.5;
    let band_half = (band_out - band_in) * 0.5;
    let has_band = band_half >= 1.8;

    // An engraved line under the rim, and brackets at the corners.
    if tier >= 3u {
        out = over(out, col_a, line(e, rim_w + 1.0, 0.7, aa) * fade * 0.6);
        let reach = 12.0 + tf * 1.3;
        let bracket = 1.0 - smoothstep(reach - 1.0, reach, max(gap.x, gap.y));
        out = over(out, col_b, line(e, rim_w + 1.2, 1.4, aa) * bracket * 0.9);
    }

    if has_band {
        if tier == 6u {
            // A standing wave, its nodes at the corners and the edges' middle.
            let wave = sin(PI * u) * cos(t * 0.9);
            let swell = 0.55 + 0.45 * sin(TAU * (ph * 3.0 - t * 0.14));
            out = over(out, col_b, line(e, band_mid + band_half * 0.75 * wave, 1.0, aa) * swell);
            out = over(out, col_a, line(e, band_mid - band_half * 0.75 * wave, 0.8, aa) * 0.8);
        }
        if tier == 7u {
            // Guilloche: two lines crossing, a grain of gold in each eye.
            let twist = sin(PI * u);
            let shimmer = 0.6 + 0.4 * pow(0.5 + 0.5 * cos(TAU * (ph * 2.0 + t * 0.06)), 4.0);
            out = over(out, col_b, line(e, band_mid + band_half * 0.8 * twist, 0.9, aa) * shimmer);
            out = over(out, col_a, line(e, band_mid - band_half * 0.8 * twist, 0.9, aa) * shimmer);
            let eye = length(vec2<f32>((in_cell - 0.5) * (span / cells), e - band_mid));
            out = over(out, col_spark, (1.0 - smoothstep(0.7 - aa, 0.7 + aa, eye)) * shimmer);
        }
        if tier == 8u {
            // Beads of ruby, lit by the beat from the edges' middle outward.
            let pitch = span / cells;
            let bead = vec2<f32>((in_cell - 0.5) * pitch, e - band_mid);
            let radius = min(band_half, pitch * 0.42);
            let round_off = 1.0 - smoothstep(radius - aa, radius + aa, length(bead));
            let shine = exp(-dot(bead + vec2<f32>(0.6, 0.7), bead + vec2<f32>(0.6, 0.7)) * 0.9);
            let run = exp(-fract(u / cells - t * 0.3) * 3.5);
            let rgb = mix(col_a * (0.7 + beat * 0.6), col_b, shine * 0.9 + beat * 0.25 + run * 0.3);
            out = over(out, rgb, round_off);
        }
        if tier == 9u {
            // A chain of cut amethyst, great and small by turns, read by a
            // slow wave of light.
            let pitch = span / cells;
            let stone_at = vec2<f32>((in_cell - 0.5) * pitch, e - band_mid);
            let great = select(0.62, 1.0, fract(cell * 0.5) < 0.25);
            let stone = min(band_half, pitch * 0.5) * great;
            let cut = 1.0 - smoothstep(stone - aa, stone + aa, abs(stone_at.x) + abs(stone_at.y));
            let face = select(0.3, 0.75, stone_at.x * stone_at.y > 0.0);
            let word = pow(0.5 + 0.5 * cos(TAU * (u / cells * 2.0 - t * 0.1)), 3.0);
            out = over(out, mix(mix(col_a, col_b, face), col_spark, word * 0.8), cut);
        }
        if tier == 10u {
            // Flames licking inward from the rim; they flow, never flicker.
            let lick = 0.5 + 0.5 * (sin(u * 1.7 + t * 1.3) * 0.55
                + sin(u * 3.3 - t * 1.9) * 0.3
                + sin(u * 0.6 + t * 0.7) * 0.15);
            let tall = (band_out - rim_w) * (0.35 + 0.65 * lick);
            let up = (e - rim_w) / max(tall, 0.5);
            let body = 1.0 - smoothstep(0.5, 1.0, up);
            let fire = mix(vec3<f32>(1.0, 0.88, 0.55), vec3<f32>(1.0, 0.40, 0.12), clamp(up, 0.0, 1.0));
            out = over(out, mix(fire, col_b, 0.2), body * 0.9);
        }
        if tier == 11u {
            // Curtains of aurora falling from the rim, motes of light in them.
            let fold = sin(u * 0.9 + t * 0.5 + 1.8 * sin(u * 0.31 - t * 0.23));
            let ray = pow(0.5 + 0.5 * sin(u * 3.4 + fold * 2.2), 2.0);
            let fall = 1.0 - clamp((e - rim_w) / max(band_out - rim_w, 0.5), 0.0, 1.0);
            let sky = mix(col_b, vec3<f32>(0.45, 0.95, 0.75), 0.5 + 0.5 * fold);
            out = over(out, sky, ray * fall * fall * 0.85);
            let pitch = span / cells;
            let seed = hash21(vec2<f32>(cell, 11.0));
            let mote_at = vec2<f32>(
                (in_cell - 0.5 - (seed - 0.5) * 0.5) * pitch,
                e - band_mid - (seed - 0.5) * band_half,
            );
            let twinkle = pow(0.5 + 0.5 * sin(t * 1.2 + seed * TAU), 6.0);
            out = over(out, col_spark, exp(-dot(mote_at, mote_at) * 1.4) * twinkle);
        }
    }

    // The gem each corner holds, from emerald up, or a boss on bare metal.
    let at_corner = vec2<f32>(gap.x, gap.y) - vec2<f32>(max(room * 0.62, 3.0));
    if tier >= 2u && tier < 5u {
        let boss = 1.0 - smoothstep(1.1 - aa, 1.1 + aa, length(at_corner));
        out = over(out, col_b, boss);
    }
    if tier >= 5u {
        let stone = 0.9 + tf * 0.28;
        let diamond = abs(at_corner.x) + abs(at_corner.y);
        let cut = 1.0 - smoothstep(stone - aa, stone + aa, diamond);
        let facet = 0.5 + 0.5 * sin(sign(at_corner.x) * 1.3 + sign(at_corner.y) * 2.1 + t * 0.7);
        out = over(out, mix(col_a, col_spark, facet), cut);
    }

    // Light that travels: the glint of polish, and comets from amethyst up.
    var travel = 0.0;
    if tier >= 4u {
        travel = comet(ph, t * lap * 1.5, 0.018) * 0.8;
    }
    if tier >= 9u {
        let head = t * 0.075;
        let tail = 0.03 + (tf - 9.0) * 0.012;
        travel = max(comet(ph, head, tail), comet(ph, head + 0.5, tail));
        if tier == 11u {
            travel = max(travel, max(comet(ph, head + 0.25, tail), comet(ph, head + 0.75, tail)));
        }
    }
    metal = mix(metal, col_spark, clamp(travel, 0.0, 1.0));
    if tier == 8u {
        metal = mix(metal, col_b, beat * 0.4);
    }

    // Favourite decks ("leicht goldig umrandet") blend a warm golden rim.
    if fav {
        let gold_a = vec3<f32>(0.82, 0.64, 0.20);
        let gold_b = vec3<f32>(1.00, 0.88, 0.46);
        let gold_sheen = 0.5 + 0.5 * sin(TAU * ph * 2.0 - t * 1.1);
        let gold = mix(gold_a, gold_b, gold_sheen);
        let fav_weight = select(0.42, 0.92, tier == 0u);
        metal = mix(metal, gold, fav_weight);
    }

    // Hover brightens the rim smoothly.
    metal = mix(metal, col_spark, hov * 0.22);

    strength = select(
        clamp(f32(fav) * 0.78 + hov * 0.55, 0.0, 1.0),
        0.84 + tf * 0.014,
        tier > 0u,
    );
    out = over(out, metal, rim_mask * strength);

    // From ruby up the corners flare in turn, one at a time.
    if tier >= 8u {
        let turn = fract(t * 0.11 + select(0.0, 0.25, p.x > 0.0) + select(0.0, 0.5, p.y > 0.0));
        let flare = pow(max(0.0, 1.0 - abs(turn - 0.5) * 7.0), 2.0);
        let rays = gleam(at_corner, 3.0 + tf * 0.5);
        out = over(out, col_spark, clamp(rays * flare, 0.0, 1.0) * fade);
    }

    return out;
}
