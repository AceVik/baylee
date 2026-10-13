// What the shells round a permanent share between the table (`shell.wesl`)
// and the preview (`shell_ui.wesl`): the card they stand round, a keyword's
// border and summoning sickness's wave. `shellmat.rs` says what each shell
// is and why it may stand where it stands; the two shaders say where it is
// seen from, the table's through its real camera and the preview's from
// straight over the card.
//
// Everything here is in a shell's own space, the card's: its middle at the
// origin, `y` up the card, `z` up off its face, in card widths. Where a term
// needs a direction in the world it says so.

/// Which shell a material draws. `shellmat::ShellKind`.
const SHELL_BORDER: u32 = 1u;
const SHELL_RING: u32 = 2u;
const SHELL_WAVE: u32 = 3u;

/// The keywords a border says, as bits (`shellmat::Ward`), and the order
/// its layers are laid in, inside out.
const WARD_INDESTRUCTIBLE: u32 = 1u;
const WARD_DEFENDER: u32 = 2u;
const WARD_HEXPROOF: u32 = 4u;
const WARD_SHROUD: u32 = 8u;

/// The card, in its own space: half its width and height, and its corner.
/// `shellmat` holds these to `CARD_WIDTH`, `CARD_HEIGHT` and `CARD_CORNER`.
const CARD_HALF_W: f32 = 0.5;
const CARD_HALF_H: f32 = 0.6985;
const CARD_ROUND: f32 = 0.0476;
/// How far off the print the mask takes to open: outward only.
const MASK_FEATHER: f32 = 0.012;
/// A standing border's top edge over the face, and its foot under it.
const RIM_RISE: f32 = 0.01;
const RIM_DROP: f32 = 0.083;
/// Summoning sickness's wave (`shellmat::WAVE_*`): how far over the face its
/// sheet lies, how high its crest rises over that, where the sheet stops
/// inside the card's edge, and its timing in seconds.
const WAVE_LIFT: f32 = 0.003;
const WAVE_CREST: f32 = 0.009;
const WAVE_INSET: f32 = 0.05;
const WAVE_TRAVEL: f32 = 2.8;
const WAVE_PERIOD: f32 = 4.2;
/// How wide the crest is, in shares of the way from the middle to the edge,
/// and how far in from the sheet's edge it settles to it.
const CREST_WIDTH: f32 = 0.07;
const EDGE_SETTLE: f32 = 0.06;
/// Where the wave is held with motion off: most of the way out.
const WAVE_STILL: f32 = 0.55;
/// The wave's moonlight, linear.
const MOONLIGHT: vec3<f32> = vec3<f32>(0.55, 0.63, 0.92);
/// The light every shell catches, from over the player's shoulder, in the
/// world.
const KEY: vec3<f32> = vec3<f32>(-0.293, 0.838, 0.461);

/// The border's colours, linear, in the blue hour's key: indestructible's
/// gold, deep and lit; defender's slate grey stone, unlike the offer's gold
/// light on the felt and the joints between its blocks;
/// hexproof's blue and the light flowing along it; shroud's violet mist and
/// its pale.
const GOLD_DEEP: vec3<f32> = vec3<f32>(0.42, 0.27, 0.08);
const GOLD: vec3<f32> = vec3<f32>(1.0, 0.74, 0.32);
const GOLD_GLINT: vec3<f32> = vec3<f32>(1.0, 0.92, 0.72);
const STONE: vec3<f32> = vec3<f32>(0.34, 0.33, 0.34);
const JOINT: vec3<f32> = vec3<f32>(0.035, 0.03, 0.03);
const HEX_BLUE: vec3<f32> = vec3<f32>(0.20, 0.50, 1.0);
const HEX_LIGHT: vec3<f32> = vec3<f32>(0.70, 0.88, 1.0);
const MIST: vec3<f32> = vec3<f32>(0.50, 0.30, 0.95);
const MIST_PALE: vec3<f32> = vec3<f32>(0.80, 0.72, 1.0);
/// How long a block of defender's wall is along the card's edge, in card
/// widths, and the share of it its joint takes: a whole number of blocks
/// round the card, so no block is cut where the outline closes.
const BLOCK: f32 = 0.11875;
const BLOCK_JOINT: f32 = 0.09;
/// How far round the card the border's light goes in a second: the gold's
/// one glint a lap every twelve seconds, hexproof's three streaks quicker,
/// shroud's mist slowest of all.
const GLINT_LAP: f32 = 0.083;
const FLOW_LAP: f32 = 0.16;
const MIST_DRIFT: f32 = 0.25;
/// How many cells of mist go round the card.
const MIST_CELLS: f32 = 43.0;
/// The card's outline, in card widths, which the border's patterns are
/// measured along; the mist's cells (43 round it) and the blocks (40) are
/// whole numbers of it, so neither breaks where the outline closes.
const OUTLINE: f32 = 4.75;

/// Signed distance to the card's rounded rectangle, in its own space.
fn card_sdf(p: vec2<f32>) -> f32 {
    let half = vec2<f32>(CARD_HALF_W, CARD_HALF_H);
    let q = abs(p) - (half - vec2<f32>(CARD_ROUND));
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - CARD_ROUND;
}

/// Which way is out from the card at `p`, in its own plane.
fn card_out(p: vec2<f32>) -> vec2<f32> {
    let q = abs(p) - (vec2<f32>(CARD_HALF_W, CARD_HALF_H) - vec2<f32>(CARD_ROUND));
    return sign(p) * normalize(max(q, vec2<f32>(1e-5)));
}


/// Value noise for shroud's mist, smooth across a cell, repeating every
/// `period` cells along `x` so it closes round the card without a seam.
fn mist_noise(p: vec2<f32>, period: f32) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let x0 = i.x - period * floor(i.x / period);
    let x1 = x0 + 1.0 - period * floor((x0 + 1.0) / period);
    let k = vec2<f32>(127.1, 311.7);
    let h = fract(sin(vec4<f32>(
        dot(vec2<f32>(x0, i.y), k),
        dot(vec2<f32>(x1, i.y), k),
        dot(vec2<f32>(x0, i.y + 1.0), k),
        dot(vec2<f32>(x1, i.y + 1.0), k),
    )) * 43758.5453);
    return mix(mix(h.x, h.y, u.x), mix(h.z, h.w, u.x), u.y);
}

/// Which keyword lies in layer `k` of a border saying `ward`, inside out:
/// indestructible, defender, then hexproof or shroud.
fn ward_style(ward: u32, k: u32) -> u32 {
    var seen = 0u;
    for (var bit = 0u; bit < 4u; bit = bit + 1u) {
        let style = 1u << bit;
        if (ward & style) != 0u {
            if seen == k {
                return style;
            }
            seen = seen + 1u;
        }
    }
    return 0u;
}

/// One layer of a keyword's border: its colour and how much of it there is,
/// `f` across the layer (0 on the card's side, 1 on the far one), `s` round
/// the card (0..1, `card_common::perimeter`), `t` the clock (stopped for
/// reduce-motion, where every pattern holds where it stands), `aa` how far
/// `f` moves across a pixel and `aa_s` how far `s` does, taken where every
/// pixel still runs. Each keyword is its pattern and its motion as
/// much as its colour, so one that cannot tell the hues apart still can.
fn ward_layer(style: u32, f: f32, s: f32, t: f32, aa: f32, aa_s: f32) -> vec4<f32> {
    // Every layer but the mist is cut off from the next by a dark hairline.
    let edge = smoothstep(0.0, 0.14 + aa, f) * smoothstep(0.0, 0.14 + aa, 1.0 - f);
    let ridge = 1.0 - abs(2.0 * f - 1.0);
    if style == WARD_INDESTRUCTIBLE {
        // A steady golden rim, rounded across, one glint going round it.
        let lap = 1.0 - 2.0 * abs(fract(s - GLINT_LAP * t) - 0.5);
        let glint = pow(lap, 28.0) * ridge;
        let colour = mix(GOLD_DEEP, GOLD, sqrt(ridge)) + GOLD_GLINT * 0.9 * glint;
        return vec4<f32>(colour, 0.95 * edge);
    }
    if style == WARD_DEFENDER {
        // A wall of stone blocks round the card, each a shade of its own,
        // their joints dark; still, as a wall is.
        let x = s * OUTLINE / BLOCK;
        let cell = floor(x);
        let joint = min(fract(x), 1.0 - fract(x));
        let aa_x = aa_s * OUTLINE / BLOCK;
        let block = smoothstep(BLOCK_JOINT * 0.5 - aa_x, BLOCK_JOINT * 0.5 + aa_x, joint);
        let shade = 0.8 + 0.4 * fract(sin(cell * 12.9898) * 43758.5453);
        let lit = 0.7 + 0.45 * (1.0 - f);
        let colour = mix(JOINT, STONE * shade * lit, block);
        return vec4<f32>(colour, 0.95 * edge);
    }
    if style == WARD_HEXPROOF {
        // A thin line of blue light, three streaks flowing round it.
        let core = exp(-pow((f - 0.5) / 0.2, 2.0));
        let flow = pow(fract(s * 3.0 - FLOW_LAP * t), 6.0);
        let colour = HEX_BLUE * (0.45 + 0.55 * core) + HEX_LIGHT * flow * core;
        return vec4<f32>(colour, (0.3 + 0.65 * core) * edge);
    }
    if style == WARD_SHROUD {
        // A soft violet mist with no edge, drifting.
        let body = smoothstep(0.0, 0.5, f) * smoothstep(0.0, 0.5, 1.0 - f);
        let fog = mist_noise(vec2<f32>(s * MIST_CELLS - MIST_DRIFT * t, f * 2.0 + 0.3 * MIST_DRIFT * t), MIST_CELLS);
        let colour = mix(MIST, MIST_PALE, fog * fog);
        return vec4<f32>(colour, body * (0.55 + 0.45 * fog));
    }
    return vec4<f32>(0.0);
}

/// A keyword's border at `across` (0 at the card's side of the whole band, 1
/// at its far side), `s` round the card: the layer that lies there, of
/// `layers`, in the order [`ward_style`] lays them.
fn ward_border(ward: u32, layers: u32, across: f32, s: f32, t: f32, aa: f32, aa_s: f32) -> vec4<f32> {
    let n = f32(max(layers, 1u));
    let x = clamp(across, 0.0, 0.9999) * n;
    let k = u32(floor(x));
    return ward_layer(ward_style(ward, k), fract(x), s, t, aa * n, aa_s);
}

/// A wave's crest at `p`, its front `along` its way out: 1 on the front,
/// falling away either side, swelling in as it starts and out as it reaches
/// the edge, and settled at the sheet's own edge.
fn crest_at(p: vec2<f32>, along: f32) -> f32 {
    // 0 on the card's middle line, 1 at its edge.
    let s = 1.0 + card_sdf(p) / CARD_HALF_W;
    let reach = 1.0 - WAVE_INSET / CARD_HALF_W;
    let out = clamp(along, 0.0, 1.0);
    let front = (1.0 - (1.0 - out) * (1.0 - out)) * reach;
    let across = (s - front) / CREST_WIDTH;
    let swell = sin(3.14159265 * out);
    let settle = smoothstep(0.0, EDGE_SETTLE, -card_sdf(p) - WAVE_INSET);
    return exp(-across * across) * swell * settle;
}

/// How a wave's sheet leans at `p`: its crest's rise per unit across the
/// card, each way.
fn crest_lean(p: vec2<f32>, along: f32) -> vec2<f32> {
    let e = 0.004;
    return WAVE_CREST / (2.0 * e) * vec2<f32>(
        crest_at(p + vec2<f32>(e, 0.0), along) - crest_at(p - vec2<f32>(e, 0.0), along),
        crest_at(p + vec2<f32>(0.0, e), along) - crest_at(p - vec2<f32>(0.0, e), along),
    );
}

/// How far out a wave's front is at `clock` (in periods, any phase added),
/// 0 at the card's middle line, 1 at its sheet's edge and past 1 while it
/// rests before the next; held still most of the way out with motion off.
fn wave_along(clock: f32, motion: f32) -> f32 {
    return select(WAVE_STILL, fract(clock) * WAVE_PERIOD / WAVE_TRAVEL, motion > 0.5);
}

/// How much moonlight a wave gives back at `p`, its front `along` its way
/// out, seen from `v` and lit by `key` (both in the card's space): on the
/// crest's slopes, more where they turn from the eye, a breath of it on its
/// top and the key mirrored along it; nothing where there is no crest, so
/// the wave passes and leaves the print as it was.
fn wave_glow(p: vec2<f32>, along: f32, v: vec3<f32>, key: vec3<f32>) -> f32 {
    let c = crest_at(p, along);
    let lean = crest_lean(p, along);
    let ln = normalize(vec3<f32>(-lean, 1.0));
    let away = 1.0 - abs(dot(ln, v));
    let mirrored = max(dot(reflect(-v, ln), key), 0.0);
    let glint = smoothstep(0.97, 0.995, mirrored) * c;
    return min(0.14 * c + length(lean) * (1.0 + 1.8 * away) + 0.6 * glint, 0.6);
}
