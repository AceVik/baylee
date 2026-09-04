// What the table's card shader and its UI twin have to agree about: the shape
// of the printed card, and the keyword rail it wears along its bottom edge.
//
// # Why this file exists
//
// A creature in the own-board overlay is the same creature as the one on the
// table, and two hundred lines of pictogram kept in step by hand would not
// stay in step. So this is plain WGSL with no bindings and no bevy syntax at
// all: every shader-global it needs — the time, the colour underneath — comes
// in as a parameter, which is also what lets `cardmat::tests` parse it on its
// own and what keeps it out of the two different bind groups the two shaders
// read `globals` from.
//
// # Why a rail and not more paint
//
// The border band is a *material* — indestructible is what the card is made
// of, hexproof and shroud are what lies over it — and a material composes
// with at most one other material before it stops saying either thing. The
// combat keywords are not like that. There are eleven of them, they are
// equal, a creature can carry six at once, and what a player needs is to
// *count* them and name them. Paint cannot count. Marks can: one slot each,
// always in the same order, so the row is read the way a row of icons is read
// and not the way a colour is guessed at.
//
// The three band keywords are deliberately absent from the rail. The border
// already says them, and a mark that repeated it would be the same claim
// twice in two languages.

/// The card's aspect, so a length measured in card widths means the same on
/// both axes.
const CARD_ASPECT: f32 = 63.0 / 88.0;

/// The printed corner radius, as a fraction of the card's width.
///
/// A Magic card is 63 × 88 mm with a 3 mm corner — 4.76% — and a Scryfall
/// scan is the whole rectangle, so everything outside that rounded rectangle
/// is the white of the scanner bed and never the card. Cutting it is not a
/// stylistic choice; it is the difference between a card and a photograph of
/// one. `table::CARD_CORNER` rounds the mesh at the same fraction, and a test
/// reads this line to make sure it still does.
const PRINTED_CORNER: f32 = 0.0476;

/// Signed distance to the printed card's rounded rectangle, in card widths.
/// Negative inside the card, positive out in the scan's white corner.
fn corner_sdf(uv: vec2<f32>) -> f32 {
    // Width-units: x spans 1.0, y spans 1/aspect.
    let half = vec2<f32>(0.5, 0.5 / CARD_ASPECT);
    let p = vec2<f32>(uv.x, uv.y / CARD_ASPECT) - half;
    let q = abs(p) - (half - vec2<f32>(PRINTED_CORNER));
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - PRINTED_CORNER;
}

/// How many keywords can ride the rail.
const MARK_COUNT: u32 = 11u;

/// Where the marks begin in the glow word, and how wide the field is.
///
/// Both card shaders do the shifting at the call site, so nothing below this
/// line knows that the flags for indestructible, hexproof, shroud, a sleeping
/// creature and the three lights this client offers live underneath. The
/// Rust half is
/// `cardmat::glow::MARK_SHIFT`, and a test reads this file to check that the
/// two still agree.
const MARK_SHIFT: u32 = 8u;
const MARK_FIELD: u32 = 0x7ffu;

/// How far the rail sits in from the printed edge, in card widths.
const RAIL_INSET: f32 = 0.052;

/// A slot's size when there is room for it, in card widths.
const RAIL_SLOT: f32 = 0.115;

/// How much of the card's width the rail may ever take.
///
/// The remaining fifth of the bottom edge is reserved, on purpose and before
/// anything needs it: power/toughness and the counter dice belong in that
/// corner, and a rail that had to move once they arrived would move on every
/// card in every screenshot ever taken of this client.
const RAIL_SPAN: f32 = 0.70;

/// The beat every mark breathes on, in radians per second.
///
/// One beat, phase-offset per slot: five marks that each animated on their
/// own timing would strobe, and thirty cards of them would be a fairground.
const BEAT: f32 = 1.15;

/// The plate the marks sit on, so ivory reads on any artwork.
const PLATE: vec3<f32> = vec3<f32>(0.045, 0.052, 0.062);

/// The ink every mark is drawn in, before its own colour is mixed into it.
const INK: vec3<f32> = vec3<f32>(0.94, 0.96, 0.99);

fn sd_circle(p: vec2<f32>, r: f32) -> f32 {
    return length(p) - r;
}

fn sd_box(p: vec2<f32>, b: vec2<f32>) -> f32 {
    let q = abs(p) - b;
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0);
}

fn sd_round_box(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    return sd_box(p, b - vec2<f32>(r)) - r;
}

/// Distance to a line segment. Every stroke in every mark is one of these,
/// which is what gives eleven pictograms one stroke width.
fn sd_segment(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    return length(pa - ba * h);
}

/// A filled triangle, as the outermost of its three edge half-planes.
///
/// Not the vertex-exact form: this one is exact along the edges and slightly
/// conservative near the corners, which at eight pixels tall is a difference
/// nobody can see and a dozen instructions nobody has to run.
fn sd_tri(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>, c: vec2<f32>) -> f32 {
    let ab = b - a;
    let bc = c - b;
    let ca = a - c;
    let da = dot(p - a, normalize(vec2<f32>(ab.y, -ab.x)));
    let db = dot(p - b, normalize(vec2<f32>(bc.y, -bc.x)));
    let dc = dot(p - c, normalize(vec2<f32>(ca.y, -ca.x)));
    return max(da, max(db, dc));
}

// ---- the eleven marks
//
// Cell coordinates run -0.5..0.5 with **y downward**, the way the card's UV
// does. Every mark stays inside a radius of about 0.36 so that neighbouring
// slots never touch, and every stroke is 0.055 wide so the row reads as one
// alphabet rather than eleven drawings.

/// Flying: a chevron lifted off the ground it no longer touches.
fn mark_flying(p: vec2<f32>, ph: f32) -> f32 {
    let lift = vec2<f32>(0.0, -0.025 * (0.5 + 0.5 * sin(ph)));
    let q = p - lift;
    let wing = min(
        sd_segment(q, vec2<f32>(-0.30, 0.02), vec2<f32>(0.0, -0.20)),
        sd_segment(q, vec2<f32>(0.0, -0.20), vec2<f32>(0.30, 0.02)),
    ) - 0.055;
    let ground = sd_segment(p, vec2<f32>(-0.17, 0.30), vec2<f32>(0.17, 0.30)) - 0.030;
    return min(wing, ground);
}

/// First strike: one blade, with the flash that arrives before the others.
fn mark_first_strike(p: vec2<f32>, ph: f32) -> f32 {
    let blade = sd_segment(p, vec2<f32>(-0.06, 0.30), vec2<f32>(0.10, -0.28)) - 0.052;
    let guard = sd_segment(p, vec2<f32>(-0.20, 0.02), vec2<f32>(0.20, -0.04)) - 0.032;
    return min(blade, guard);
}

/// Double strike: the same blade twice, because that is what it is.
fn mark_double_strike(p: vec2<f32>, ph: f32) -> f32 {
    let a = sd_segment(p, vec2<f32>(-0.20, 0.30), vec2<f32>(-0.04, -0.28)) - 0.045;
    let b = sd_segment(p, vec2<f32>(0.04, 0.30), vec2<f32>(0.20, -0.28)) - 0.045;
    return min(a, b);
}

/// Deathtouch: a drop of something that only has to land once.
fn mark_deathtouch(p: vec2<f32>, ph: f32) -> f32 {
    let swell = 1.0 + 0.06 * sin(ph);
    let q = p / swell;
    let bulb = sd_circle(q - vec2<f32>(0.0, 0.11), 0.21);
    let tip = sd_tri(
        q,
        vec2<f32>(0.0, -0.32),
        vec2<f32>(0.19, 0.16),
        vec2<f32>(-0.19, 0.16),
    );
    return min(bulb, tip) * swell;
}

/// Haste: a head with the trail it has already left behind.
fn mark_haste(p: vec2<f32>, ph: f32) -> f32 {
    let run = 0.03 * sin(ph);
    let q = p - vec2<f32>(run, 0.0);
    let head = sd_circle(q - vec2<f32>(0.16, 0.0), 0.115);
    let t1 = sd_segment(q, vec2<f32>(-0.30, -0.13), vec2<f32>(0.04, -0.10)) - 0.036;
    let t2 = sd_segment(q, vec2<f32>(-0.34, 0.02), vec2<f32>(0.02, 0.0)) - 0.036;
    let t3 = sd_segment(q, vec2<f32>(-0.26, 0.16), vec2<f32>(0.04, 0.11)) - 0.036;
    return min(head, min(t1, min(t2, t3)));
}

/// Lifelink: a heart, on a beat that thumps twice and rests.
fn mark_lifelink(p: vec2<f32>, ph: f32) -> f32 {
    let beat = pow(0.5 + 0.5 * sin(ph), 6.0);
    let q = p / (1.0 + 0.09 * beat);
    let l = sd_circle(q - vec2<f32>(-0.14, -0.07), 0.16);
    let r = sd_circle(q - vec2<f32>(0.14, -0.07), 0.16);
    let v = sd_tri(
        q,
        vec2<f32>(0.0, 0.32),
        vec2<f32>(-0.29, -0.06),
        vec2<f32>(0.29, -0.06),
    );
    return min(min(l, r), v);
}

/// Menace: two rings, because one of them is never enough.
fn mark_menace(p: vec2<f32>, ph: f32) -> f32 {
    let sway = 0.012 * sin(ph);
    let l = abs(sd_circle(p - vec2<f32>(-0.12 - sway, 0.0), 0.17)) - 0.050;
    let r = abs(sd_circle(p - vec2<f32>(0.12 + sway, 0.0), 0.17)) - 0.050;
    return min(l, r);
}

/// Reach: the same chevron flying wears, on a stem that never leaves the
/// ground — it does not fly, it gets there.
fn mark_reach(p: vec2<f32>, ph: f32) -> f32 {
    let tip = 0.02 * sin(ph);
    let head = min(
        sd_segment(p, vec2<f32>(-0.22, -0.04 - tip), vec2<f32>(0.0, -0.26 - tip)),
        sd_segment(p, vec2<f32>(0.0, -0.26 - tip), vec2<f32>(0.22, -0.04 - tip)),
    ) - 0.052;
    let stem = sd_segment(p, vec2<f32>(0.0, -0.20), vec2<f32>(0.0, 0.24)) - 0.050;
    let foot = sd_segment(p, vec2<f32>(-0.16, 0.28), vec2<f32>(0.16, 0.28)) - 0.030;
    return min(head, min(stem, foot));
}

/// Trample: a wedge coming down on ground that has already given way.
fn mark_trample(p: vec2<f32>, ph: f32) -> f32 {
    let stomp = 0.03 * pow(0.5 + 0.5 * sin(ph), 4.0);
    let wedge = sd_tri(
        p - vec2<f32>(0.0, stomp),
        vec2<f32>(0.0, 0.16),
        vec2<f32>(-0.26, -0.24),
        vec2<f32>(0.26, -0.24),
    );
    let l = sd_segment(p, vec2<f32>(-0.30, 0.30), vec2<f32>(-0.09, 0.30)) - 0.032;
    let r = sd_segment(p, vec2<f32>(0.09, 0.30), vec2<f32>(0.30, 0.30)) - 0.032;
    return min(wedge, min(l, r));
}

/// Vigilance: an open eye, which blinks rarely and briefly.
fn mark_vigilance(p: vec2<f32>, ph: f32) -> f32 {
    let blink = smoothstep(0.94, 0.99, fract(ph * 0.06));
    let open = 1.0 - 0.85 * blink;
    let q = vec2<f32>(p.x, p.y / max(open, 0.08));
    let lens = max(
        sd_circle(q - vec2<f32>(0.0, 0.46), 0.62),
        sd_circle(q - vec2<f32>(0.0, -0.46), 0.62),
    );
    let ring = abs(lens) - 0.048;
    let pupil = sd_circle(q, 0.10);
    return min(ring, pupil);
}

/// Defender: a shield, and the only mark that does not move.
fn mark_defender(p: vec2<f32>, ph: f32) -> f32 {
    let outer = max(
        sd_box(p - vec2<f32>(0.0, -0.10), vec2<f32>(0.25, 0.19)),
        sd_circle(p - vec2<f32>(0.0, -0.34), 0.52),
    );
    let inner = max(
        sd_box(p - vec2<f32>(0.0, -0.10), vec2<f32>(0.16, 0.13)),
        sd_circle(p - vec2<f32>(0.0, -0.31), 0.40),
    );
    return max(outer, -inner);
}

/// The distance field for one mark, by slot.
fn mark_sdf(which: u32, p: vec2<f32>, ph: f32) -> f32 {
    switch which {
        case 0u: { return mark_flying(p, ph); }
        case 1u: { return mark_first_strike(p, ph); }
        case 2u: { return mark_double_strike(p, ph); }
        case 3u: { return mark_deathtouch(p, ph); }
        case 4u: { return mark_haste(p, ph); }
        case 5u: { return mark_lifelink(p, ph); }
        case 6u: { return mark_menace(p, ph); }
        case 7u: { return mark_reach(p, ph); }
        case 8u: { return mark_trample(p, ph); }
        case 9u: { return mark_vigilance(p, ph); }
        case 10u: { return mark_defender(p, ph); }
        default: { return 1.0; }
    }
}

/// Each mark's own colour, used for its halo and mixed into its ink.
///
/// Bright enough to survive the plate, and far enough apart that the row
/// still separates once the marks are too small to read as drawings — which
/// is the honest failure mode: at board scale eleven keywords are eleven
/// coloured pips, and zooming in turns them back into pictures.
fn mark_color(which: u32) -> vec3<f32> {
    switch which {
        case 0u: { return vec3<f32>(0.62, 0.80, 0.98); }
        case 1u: { return vec3<f32>(0.88, 0.92, 0.98); }
        case 2u: { return vec3<f32>(0.98, 0.86, 0.60); }
        case 3u: { return vec3<f32>(0.42, 0.82, 0.36); }
        case 4u: { return vec3<f32>(0.99, 0.52, 0.26); }
        case 5u: { return vec3<f32>(0.98, 0.46, 0.52); }
        case 6u: { return vec3<f32>(0.70, 0.52, 0.95); }
        case 7u: { return vec3<f32>(0.55, 0.86, 0.60); }
        case 8u: { return vec3<f32>(0.86, 0.66, 0.34); }
        case 9u: { return vec3<f32>(0.78, 0.92, 1.00); }
        case 10u: { return vec3<f32>(0.66, 0.74, 0.84); }
        default: { return INK; }
    }
}

/// Draws the rail over `color` and returns what is left.
///
/// `bits` is the eleven-bit mark field, already shifted down out of the glow
/// word: this file never sees the engine's keyword numbering, or the client's
/// either. `t` is `globals.time`, which the two shaders read from two
/// different bind groups — the reason it is a parameter and not a binding.
fn mark_layer(uv: vec2<f32>, bits: u32, t: f32, color: vec3<f32>) -> vec3<f32> {
    // Counted in a loop bound at compile time, and not with `countOneBits`.
    // naga lowers that to GLSL's `bitCount`, which arrived in ES 3.10, and it
    // lowers it *unguarded* — WebGL2 compiles ES 3.00, so the browser would
    // reject this shader, the card pipeline would fail to build, and the
    // table would draw no cards at all. The rail has to walk these eleven
    // bits below in any case.
    var n = 0u;
    for (var i = 0u; i < MARK_COUNT; i = i + 1u) {
        if (bits & (1u << i)) != 0u {
            n = n + 1u;
        }
    }
    if n == 0u {
        return color;
    }

    // Width-units, so a slot is square and a length means one thing.
    let p = vec2<f32>(uv.x, uv.y / CARD_ASPECT);
    let height = 1.0 / CARD_ASPECT;
    let slot = min(RAIL_SLOT, RAIL_SPAN / f32(n));
    let x0 = RAIL_INSET;
    let x1 = x0 + f32(n) * slot;
    let y1 = height - RAIL_INSET;
    let y0 = y1 - slot;

    // The antialiasing width, taken once and in uniform control flow — the
    // slot a fragment lands in is not uniform, and a derivative asked for
    // inside that branch is undefined on half the backends we ship to.
    let aa = max(fwidth(p.x), 0.0015);

    // The plate: without it a pale mark disappears into pale artwork, and
    // the row would be legible on some cards and not others.
    let mid = vec2<f32>((x0 + x1) * 0.5, (y0 + y1) * 0.5);
    let half = vec2<f32>((x1 - x0) * 0.5 + 0.014, slot * 0.5 + 0.014);
    let plate = sd_round_box(p - mid, half, slot * 0.30);
    var out = mix(color, PLATE, (1.0 - smoothstep(-aa, aa, plate)) * 0.62);

    if p.x < x0 || p.x > x1 || p.y < y0 || p.y > y1 {
        return out;
    }

    let k = u32(floor((p.x - x0) / slot));

    // The k-th mark this card actually carries. Eleven iterations, bounded at
    // compile time, no dynamic indexing: the whole reason the rail is a
    // bitfield and not a list.
    var seen = 0u;
    var which = MARK_COUNT;
    for (var i = 0u; i < MARK_COUNT; i = i + 1u) {
        if (bits & (1u << i)) != 0u {
            if seen == k {
                which = i;
                break;
            }
            seen = seen + 1u;
        }
    }
    if which == MARK_COUNT {
        return out;
    }

    let cell = vec2<f32>((p.x - x0) / slot - f32(k), (p.y - y0) / slot) - vec2<f32>(0.5);
    let phase = t * BEAT + f32(k) * 0.22;
    let d = mark_sdf(which, cell, phase);
    let accent = mark_color(which);
    let ink = mix(INK, accent, 0.55) * (0.90 + 0.10 * sin(phase));

    // Cell units, so the edge is as soft on a card in the preview as on one
    // across the table.
    let e = max(aa / slot, 0.02);
    let halo = exp(-max(d, 0.0) * 9.0) * 0.45;
    out = out + accent * halo;
    out = mix(out, ink, 1.0 - smoothstep(-e, e, d));
    return out;
}

// ------------------------------------------------------------------ the plate
//
// The bottom-right corner the rail has been reserving: a creature's power and
// toughness with the damage marked on it, or a planeswalker's loyalty. The
// Rust half is `baylee_client_core::cardplate`, which is where the numbers are
// packed and where every constant below is mirrored and tested.

/// The plate's inset, width, height and inner margin, in card widths.
const PLATE_INSET: f32 = 0.052;
const PLATE_W: f32 = 0.196;
const PLATE_H: f32 = 0.115;
const PLATE_PAD: f32 = 0.014;

/// How the packed word is read: three ten-bit numbers, two kind bits on top.
const PLATE_KIND_SHIFT: u32 = 30u;
const PLATE_SLOT_BITS: u32 = 10u;
const PLATE_SLOT_MASK: u32 = 0x3ffu;
const PLATE_BIAS: i32 = 128;

const PLATE_NONE: u32 = 0u;
const PLATE_FIGHT: u32 = 1u;
const PLATE_LOYALTY: u32 = 2u;

/// The glyph grid, and the twelve stencils drawn on it.
const GLYPH_W: u32 = 4u;
const GLYPH_H: u32 = 6u;
const GLYPH_0: u32 = 0x699996u;
const GLYPH_1: u32 = 0xe444c4u;
const GLYPH_2: u32 = 0xf42196u;
const GLYPH_3: u32 = 0x69161eu;
const GLYPH_4: u32 = 0x22fa62u;
const GLYPH_5: u32 = 0x691e8fu;
const GLYPH_6: u32 = 0x699e86u;
const GLYPH_7: u32 = 0x44221fu;
const GLYPH_8: u32 = 0x699696u;
const GLYPH_9: u32 = 0x617996u;
const GLYPH_MINUS: u32 = 0xe000u;
const GLYPH_SLASH: u32 = 0x884211u;

/// A planeswalker's rim and ink. Gilt, and explicitly not a shield: the
/// shield-shaped loyalty box is the printed planeswalker frame's own element,
/// and "a plain shield nobody owns" is the argument every borrowed frame
/// element makes (`docs/legal.md` §2).
const GILT: vec3<f32> = vec3<f32>(0.87, 0.73, 0.38);

/// Marked damage, which is the one thing on this plate that is not printed on
/// a real card — so it is drawn as a rising fill rather than as a numeral.
const EMBER: vec3<f32> = vec3<f32>(0.88, 0.27, 0.18);

fn glyph_word(which: u32) -> u32 {
    switch which {
        case 0u: { return GLYPH_0; }
        case 1u: { return GLYPH_1; }
        case 2u: { return GLYPH_2; }
        case 3u: { return GLYPH_3; }
        case 4u: { return GLYPH_4; }
        case 5u: { return GLYPH_5; }
        case 6u: { return GLYPH_6; }
        case 7u: { return GLYPH_7; }
        case 8u: { return GLYPH_8; }
        case 9u: { return GLYPH_9; }
        case 10u: { return GLYPH_MINUS; }
        default: { return GLYPH_SLASH; }
    }
}

/// One cell of a glyph, and 0 outside it — which is what stops a stencil
/// bleeding into the one beside it when the grid is sampled smoothly.
fn glyph_cell(word: u32, col: i32, row: i32) -> f32 {
    if col < 0 || col >= i32(GLYPH_W) || row < 0 || row >= i32(GLYPH_H) {
        return 0.0;
    }
    // Bit `3 - column`, which is what lets the Rust literals be read as the
    // pictures they draw.
    let bit = u32(row) * 4u + (3u - u32(col));
    return f32((word >> bit) & 1u);
}

/// How many decimal digits a number is drawn in. Three is the ceiling the
/// packing allows, so this needs no fourth case.
fn plate_digits(v: u32) -> u32 {
    if v >= 100u { return 3u; }
    if v >= 10u { return 2u; }
    return 1u;
}

/// The `i`-th digit of `v` from the left, given it is drawn in `n` of them.
///
/// The loop is bounded at two because three digits is the ceiling — the same
/// discipline as the rail's eleven: WebGL2 wants every bound at compile time.
fn plate_digit_at(v: u32, n: u32, i: u32) -> u32 {
    var p = 1u;
    for (var k = 0u; k < 2u; k = k + 1u) {
        if k + i + 1u < n {
            p = p * 10u;
        }
    }
    return (v / p) % 10u;
}

/// Draws the plate over `color` and returns what is left.
///
/// `word` is `cardplate::Plate::packed`. Not gated on whether the card has
/// artwork: a card drawn as a flat tint is a card whose art has not loaded,
/// and its body is the thing a player most needs off it.
fn plate_layer(uv: vec2<f32>, word: u32, color: vec3<f32>) -> vec3<f32> {
    let kind = word >> PLATE_KIND_SHIFT;
    if kind == PLATE_NONE {
        return color;
    }

    // Width units, so a length means the same thing on both axes.
    let p = vec2<f32>(uv.x, uv.y / CARD_ASPECT);
    let height = 1.0 / CARD_ASPECT;

    // Every derivative this function takes, taken here — the branches below
    // are not uniform, and a derivative asked for inside one of them is
    // undefined on half the backends this ships to.
    let aa = max(fwidth(p.x), 0.0015);

    let x1 = 1.0 - PLATE_INSET;
    let x0 = x1 - PLATE_W;
    let y1 = height - PLATE_INSET;
    let y0 = y1 - PLATE_H;
    let mid = vec2<f32>((x0 + x1) * 0.5, (y0 + y1) * 0.5);
    let half = vec2<f32>(PLATE_W * 0.5, PLATE_H * 0.5);

    let d_plate = sd_round_box(p - mid, half, PLATE_H * 0.28);
    let inside = 1.0 - smoothstep(-aa, aa, d_plate);
    if inside <= 0.0 {
        return color;
    }
    var out = mix(color, PLATE, inside * 0.88);

    let a = i32(word & PLATE_SLOT_MASK) - PLATE_BIAS;
    let b = i32((word >> PLATE_SLOT_BITS) & PLATE_SLOT_MASK) - PLATE_BIAS;
    let c = i32((word >> (PLATE_SLOT_BITS * 2u)) & PLATE_SLOT_MASK) - PLATE_BIAS;

    // Damage rises from the bottom of the plate to `damage / toughness`, so
    // what a player reads is how close to lethal this creature is rather than
    // an arithmetic problem in two numerals.
    if kind == PLATE_FIGHT && c > 0 && b > 0 {
        let frac = clamp(f32(c) / f32(b), 0.0, 1.0);
        let level = y1 - PLATE_H * frac;
        let fill = inside * smoothstep(level - aa, level + aa, p.y);
        out = mix(out, EMBER, fill * 0.58);
    }

    // The rim. Gilt for a planeswalker, which is the whole of how the two
    // plates are told apart — same corner, same shape, same numeral role.
    let rim = 1.0 - smoothstep(-aa, aa, abs(d_plate) - 0.0045);
    var accent = INK;
    if kind == PLATE_LOYALTY {
        accent = GILT;
    }
    out = mix(out, accent, rim * 0.55);

    // How many glyphs, and therefore how big they are: a lone loyalty numeral
    // fills the plate's height, a `10/10` shrinks to fit its width. Shrinking
    // rather than clipping is the degradation that stays honest — a plate that
    // cut a digit off would be showing a number that is wrong.
    let neg = kind == PLATE_FIGHT && a < 0;
    let av = u32(abs(a));
    let bv = u32(max(b, 0));
    let da = plate_digits(av);
    let db = plate_digits(bv);
    let lead = select(0u, 1u, neg);
    var n = da;
    if kind == PLATE_FIGHT {
        n = lead + da + 1u + db;
    }

    let span = f32(n * GLYPH_W + (n - 1u));
    let unit = min(
        (PLATE_W - 2.0 * PLATE_PAD) / span,
        (PLATE_H - 2.0 * PLATE_PAD) / f32(GLYPH_H),
    );
    let text = vec2<f32>(span * unit, f32(GLYPH_H) * unit);
    let local = (p - (mid - text * 0.5)) / unit;
    if local.x < 0.0 || local.y < 0.0 || local.y >= f32(GLYPH_H) {
        return out;
    }

    let stride = f32(GLYPH_W + 1u);
    let k = u32(floor(local.x / stride));
    if k >= n {
        return out;
    }
    let col = local.x - f32(k) * stride;
    if col >= f32(GLYPH_W) {
        return out;
    }

    var which = 11u;
    if kind == PLATE_LOYALTY {
        which = plate_digit_at(av, da, k);
    } else if k < lead {
        which = 10u;
    } else if k < lead + da {
        which = plate_digit_at(av, da, k - lead);
    } else if k > lead + da {
        which = plate_digit_at(bv, db, k - lead - da - 1u);
    }
    let gw = glyph_word(which);

    // The grid, sampled smoothly rather than tested. A stroke is one cell
    // wide, so bilinear over the four cells around a point peaks at 1 in the
    // middle of the stroke and reaches 0.5 at its edge — which is a stencil
    // with soft sides at any size, and one that never shows a staircase on a
    // card lying at CAMERA_LEAN.
    let g = vec2<f32>(col, local.y) - vec2<f32>(0.5);
    let base = floor(g);
    let f = g - base;
    let cx = i32(base.x);
    let cy = i32(base.y);
    let s0 = mix(glyph_cell(gw, cx, cy), glyph_cell(gw, cx + 1, cy), f.x);
    let s1 = mix(glyph_cell(gw, cx, cy + 1), glyph_cell(gw, cx + 1, cy + 1), f.x);
    let v = mix(s0, s1, f.y);
    let e = max(aa / unit, 0.06);
    return mix(out, accent, smoothstep(0.5 - e, 0.5 + e, v));
}
