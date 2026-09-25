// What the table's card shader and its UI twin have to agree about: the shape
// of the printed card, and everything this client draws beside a print —
// the keyword strip, the count, the offer's light on the felt.
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
// # Why a strip of marks and not paint
//
// The keywords are equal, a creature can carry six at once, and what a player
// needs is to *count* them and name them. Paint cannot count. Marks can: one
// slot each, always in the same order, so the row is read the way a row of
// icons is read and not the way a colour is guessed at.
//
// The strip is drawn here (`label_strip`) but not by the card: it is an
// object of its own lying on the card, with its own quad and material
// (`marks.wgsl`, `marks_ui.wgsl`), which import this file. Since #298 it is
// the card's label as well — the plate, the moon and the crests stand on it
// beside the marks — because the frame that carried them is gone.

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

// ---- the print fills the card
//
// Nothing this client *paints* on the print (#274, `docs/legal.md` §3).
// Scryfall's terms ask that a card image is not covered, cropped, tinted or
// stamped, and the artist's name and the copyright line run along the
// print's bottom edge. #274 drew a frame round the print for everything this
// client says about a card; the owner did not want the frame (#298), so the
// print fills the quad again, 1 × 1/`CARD_ASPECT` card widths, and what the
// frame said stands on objects of their own: the keyword strip lying on the
// art (`label_strip`), the count hanging off the corner (`count_badge`) and
// the offer's light on the felt round the card (`floor_light`). The one
// thing drawn on the print is its own finish (`print_finish`), because a foil
// is what that printing *is*; and light that passes over the whole card and
// leaves nothing behind — the lamp, the arrival sweep, a door.

// ---- the text face (#259)
//
// What stands in the window for a card with no print: a token, a printing
// whose art has not arrived, a card the player asked to read as text. It is
// laid out the way a card is — a border, a name bar, an art box, a type bar
// on the keyword strip's seam, a text box — so a text card's bars line up
// with its printed neighbours' in a lane, and it is light where a card is
// light, so a row of text tokens is not a row of holes. The text is not
// drawn here: the table's `Text2d` lines and the overlay's nodes stand on
// these bars. No power and toughness in here, and no state: the strip's
// plate is the P/T box, and the strip says the rest, as it does for a print.
// `textface` in client-core is the Rust half of every number here.

/// The dark border inside the window, in card widths.
const TEXT_BORDER: f32 = 0.04;
/// Where the type bar's top stands: the keyword strip's seam
/// (`cardrail::strip_bottom`).
const TEXT_SEAM: f32 = 0.7764706;
/// The step the word carries a bar's depth in (`textface::DEPTH_STEP`).
const DEPTH_STEP: f32 = 1.0 / 512.0;
const TEXT_PINLINE: f32 = 0.006;
const TEXT_BOX_GAP: f32 = 0.012;
/// Where the text box ends; under it the foot is the border's colour.
const TEXT_FOOT: f32 = 1.264;
const BAR_CORNER: f32 = 0.012;

/// The bars' colours, in linear light, in `textface::Hue`'s order.
const FACE_WHITE: vec3<f32> = vec3<f32>(0.7874, 0.7293, 0.5705);
const FACE_BLUE: vec3<f32> = vec3<f32>(0.2633, 0.448, 0.7106);
const FACE_BLACK: vec3<f32> = vec3<f32>(0.2429, 0.2234, 0.2633);
const FACE_RED: vec3<f32> = vec3<f32>(0.7484, 0.2957, 0.196);
const FACE_GREEN: vec3<f32> = vec3<f32>(0.2957, 0.5382, 0.2957);
const FACE_GOLD: vec3<f32> = vec3<f32>(0.7106, 0.5071, 0.1473);
const FACE_GREY: vec3<f32> = vec3<f32>(0.448, 0.448, 0.42);
/// The text box is the bars' colour mixed this far towards a warm white.
const PAPER_WHITE: vec3<f32> = vec3<f32>(0.8481, 0.8276, 0.7484);
const PAPER_MIX: f32 = 0.65;
/// The art box: the bars' colour this dark at its top and at its foot.
const ART_TOP: f32 = 0.35;
const ART_FOOT: f32 = 0.2;
/// How far the art box's cloth lifts and sinks it.
const CLOTH: f32 = 0.04;
const BORDER_INK: vec3<f32> = vec3<f32>(0.0049, 0.0049, 0.006);
/// What a bar's top edge gains and its bottom edge loses.
const BEVEL: f32 = 0.06;

/// `textface::face_word`'s bits.
const FACE_ON: u32 = 1u;
const FACE_BARS_SHIFT: u32 = 4u;
const FACE_NAME_SHIFT: u32 = 16u;
const FACE_TYPE_SHIFT: u32 = 24u;

/// A hue code's colour.
fn face_hue(code: u32) -> vec3<f32> {
    switch code {
        case 0u: { return FACE_WHITE; }
        case 1u: { return FACE_BLUE; }
        case 2u: { return FACE_BLACK; }
        case 3u: { return FACE_RED; }
        case 4u: { return FACE_GREEN; }
        case 5u: { return FACE_GOLD; }
        default: { return FACE_GREY; }
    }
}

/// Signed distance to a rounded part `[x0, y0, x1, y1]` of the face.
fn face_part_sdf(p: vec2<f32>, r: vec4<f32>) -> f32 {
    let half = (r.zw - r.xy) * 0.5;
    return sd_round_box(p - r.xy - half, half, BAR_CORNER);
}

/// How much of this fragment is the part: antialiased across its edge.
fn face_part(p: vec2<f32>, r: vec4<f32>, aa: f32) -> f32 {
    return 1.0 - smoothstep(-aa, aa, face_part_sdf(p, r));
}

/// A bar in its colour, lit along its top edge and shaded along its foot, so
/// it reads as a raised plate.
fn face_bar(p: vec2<f32>, r: vec4<f32>, color: vec3<f32>, aa: f32) -> vec3<f32> {
    let lit = 1.0 - smoothstep(0.0, 1.5 * aa, p.y - r.y);
    let shaded = 1.0 - smoothstep(0.0, 1.5 * aa, r.w - p.y);
    return max(color + vec3<f32>(BEVEL * (lit - shaded)), vec3<f32>(0.0));
}

/// The face at `uv`, in linear light: the whole card, border included.
/// `word` is `textface::face_word`.
fn text_face(uv: vec2<f32>, word: u32) -> vec3<f32> {
    let p = vec2<f32>(uv.x, uv.y / CARD_ASPECT);
    let aa = max(fwidth(p.x), 0.0015);
    let x0 = TEXT_BORDER;
    let x1 = 1.0 - TEXT_BORDER;
    let top = TEXT_BORDER;
    let name_end = top + f32((word >> FACE_NAME_SHIFT) & 0xffu) * DEPTH_STEP;
    let type_end = TEXT_SEAM + f32((word >> FACE_TYPE_SHIFT) & 0xffu) * DEPTH_STEP;
    let bars = face_hue((word >> FACE_BARS_SHIFT) & 0xfu);

    var out = BORDER_INK;

    // The art box: the colour dark where a print has its picture, running
    // from one colour to the other on a two-colour card, with a cloth over
    // it — the same pattern on every card, so a lane of tokens is one weave.
    let art_box = vec4<f32>(x0, name_end + TEXT_PINLINE, x1, TEXT_SEAM);
    let across = clamp((p.x - x0) / (x1 - x0), 0.0, 1.0);
    let down = clamp((p.y - art_box.y) / (art_box.w - art_box.y), 0.0, 1.0);
    let hue = mix(
        face_hue((word >> (FACE_BARS_SHIFT + 4u)) & 0xfu),
        face_hue((word >> (FACE_BARS_SHIFT + 8u)) & 0xfu),
        smoothstep(0.25, 0.75, across),
    );
    let weave = (noise(p * 3.0) - 0.5) * 1.333 + (noise(p * 9.0) - 0.5) * 0.667;
    let art = max(hue * mix(ART_TOP, ART_FOOT, down) + vec3<f32>(weave * CLOTH), vec3<f32>(0.0));
    out = mix(out, art, face_part(p, art_box, aa));

    let name_bar = vec4<f32>(x0, top, x1, name_end);
    out = mix(out, face_bar(p, name_bar, bars, aa), face_part(p, name_bar, aa));
    let type_bar = vec4<f32>(x0, TEXT_SEAM, x1, type_end);
    out = mix(out, face_bar(p, type_bar, bars, aa), face_part(p, type_bar, aa));

    let text_box = vec4<f32>(x0, type_end + TEXT_BOX_GAP, x1, TEXT_FOOT);
    out = mix(out, mix(bars, PAPER_WHITE, PAPER_MIX), face_part(p, text_box, aa));
    return out;
}

// ---- the offer's light on the felt
//
// What this client offers to do with a card, or has just been told to, is
// light: the travelling invitation, the armed card, the tap a payment plan
// will make. It lit the frame's rim from the card's edge inwards until #298
// took the frame away; now it lies on the felt round the card, on a quad of
// its own under it (`floor.wgsl`).

/// The `cardmat::glow` bits this file reads: what this client is offering to
/// do with a card, which the felt round it says (`floor_light`). `cardmat`
/// is the other half, and a test compares the two.
const GLOW_ACTIVATABLE: u32 = 8u;
const GLOW_ARMED: u32 = 32u;
const GLOW_WILL_TAP: u32 = 64u;
const GLOW_REACHABLE: u32 = 8388608u;

/// How far past the card's edge an offer's light reaches on the felt, and
/// the band next to the edge it keeps at full strength, in card widths.
///
/// Off the card entirely (#298): it used to light the frame's rim inwards,
/// and there is no frame. On the felt, the card lying on the light hides
/// every part of it that would be under the print, and so does the next
/// card of a fanned lane — which the owner accepted for rings and fans.
const FLOOR_REACH: f32 = 0.10;
const FLOOR_HARD: f32 = 0.02;

/// What the travelling activatable light averages to over its own circuit.
///
/// The chase is a band `pow(1 - 2·dist, 5)` wide riding on a floor of 0.22,
/// scaled by 0.60: its mean over a circuit is 0.22 + 0.60 / 6 = 0.32. A still
/// card (`motion == 0`) is lit at that mean rather than at a frozen crest, so
/// the offer is as strong held still as it is on average when moving.
const CHASE_STILL: f32 = 0.32;

fn hash21(p: vec2<f32>) -> f32 {
    var q = fract(p * vec2<f32>(123.34, 456.21));
    q += dot(q, q + 45.32);
    return fract(q.x * q.y);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Position around the card's edge, 0..1, clockwise from the top-left corner.
// Continuous across all four corners, so a light travelling on it runs round
// the card instead of jumping at the edges; a point off the card is placed by
// the edge it is nearest.
fn perimeter(uv: vec2<f32>) -> f32 {
    let d = vec2<f32>(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));
    if d.x < d.y {
        if uv.x < 0.5 {
            return 0.75 + (1.0 - clamp(uv.y, 0.0, 1.0)) * 0.25;
        }
        return 0.25 + clamp(uv.y, 0.0, 1.0) * 0.25;
    }
    if uv.y < 0.5 {
        return clamp(uv.x, 0.0, 1.0) * 0.25;
    }
    return 0.5 + (1.0 - clamp(uv.x, 0.0, 1.0)) * 0.25;
}

/// The light an offer or a deed throws on the felt round a card, in linear
/// light for an additive blend: `uv` is the card's own UV, running on past
/// its edges, and the light is nothing under the card.
///
/// The colours and the motion the frame's rim had: amber chasing round a
/// card that can be activated, indigo round one the client can pay for by
/// tapping lands first, a steady breathing gold round an armed one, and a
/// slow blue pulse round a land a plan will tap. `t` is the card's clock
/// (`globals.time * motion`), `m` the motion, and `now` the unscaled clock
/// for the one term that has to keep running on a still card.
fn floor_light(uv: vec2<f32>, glow: u32, t: f32, m: f32, now: f32) -> vec3<f32> {
    let d = corner_sdf(uv);
    let band = (1.0 - smoothstep(FLOOR_HARD, FLOOR_REACH, d)) * step(0.0, d);
    var out = vec3<f32>(0.0);
    if (glow & GLOW_ACTIVATABLE) != 0u {
        let head = fract(perimeter(uv) - t * 0.22);
        let chase = pow(1.0 - min(head, 1.0 - head) * 2.0, 5.0);
        let amount = mix(CHASE_STILL, 0.22 + 0.60 * chase, m);
        out = out + vec3<f32>(0.99, 0.78, 0.34) * band * amount;
    }
    if (glow & GLOW_REACHABLE) != 0u {
        let head = fract(perimeter(uv) - t * 0.22);
        let chase = pow(1.0 - min(head, 1.0 - head) * 2.0, 5.0);
        let amount = mix(CHASE_STILL, 0.22 + 0.60 * chase, m);
        out = out + vec3<f32>(0.62, 0.56, 1.00) * band * amount;
    }
    if (glow & GLOW_ARMED) != 0u {
        let hold = 0.86 + 0.14 * sin(t * 2.2);
        out = out + vec3<f32>(1.00, 0.87, 0.54) * pow(band, 0.45) * 0.52 * hold;
    }
    // On the unscaled clock, damped by the motion rather than stopped by it:
    // a plan's taps have to read as *pending* on a still table too, and a
    // pulse frozen at a trough would not say anything.
    if (glow & GLOW_WILL_TAP) != 0u {
        let pulse = 0.70 + 0.30 * sin(now * 2.2 - 0.9) * m;
        out = out + vec3<f32>(0.56, 0.60, 0.98) * band * 0.40 * pulse;
    }
    return out;
}

/// A restrained coating shaped by the printed image. Screen blending keeps
/// highlights bounded; black ink and white rules boxes retain their contrast.
/// Derivatives reuse the existing artwork sample, with no extra texture reads.
fn print_finish(
    printed: vec4<f32>, uv: vec2<f32>, angle: f32,
    time: f32, finish: u32, strength: f32,
) -> vec4<f32> {
    let rgb = clamp(printed.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
    let luma = dot(rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let chroma = max(rgb.r, max(rgb.g, rgb.b)) - min(rgb.r, min(rgb.g, rgb.b));
    // Evaluated before any branch, including the plain-card early return.
    let contour = smoothstep(0.015, 0.18, length(fwidth(rgb)));
    let ink = smoothstep(0.015, 0.10, luma);
    let paper = 1.0 - smoothstep(0.55, 0.90, luma);
    let response = ink * paper * clamp(strength, 0.0, 1.0);
    let glint = pow(1.0 - clamp(abs(angle), 0.0, 1.0), 2.0);
    let travel = sin(time * 0.48 + angle * 2.4) * 0.8 + 0.65;
    let band = exp(-18.0 * pow(uv.x * 0.7 + uv.y - travel, 2.0));
    let edge = 1.0 - smoothstep(0.004, 0.020, abs(corner_sdf(uv)));
    let edge_light = edge * (0.25 + 0.75 * band) * clamp(strength, 0.0, 1.0);
    var light = vec3<f32>(0.0);
    if finish == 1u {
        // Pigment shifts the interference phase and the reflected hue, so
        // different artwork cannot receive the same generic rainbow wash.
        let pigment = dot(rgb, vec3<f32>(0.31, 0.53, 0.79));
        let phase = (uv.x + uv.y) * 0.56 + angle * 0.84
            + time * 0.065 + pigment * 0.48;
        let rainbow = 0.5 + 0.5 * cos(6.283185 *
            (vec3<f32>(phase) + vec3<f32>(0.0, 0.3333, 0.6667)));
        let sheen = mix(vec3<f32>(0.66), rainbow, 0.68);
        let tint = mix(sheen, rgb, 0.22);
        let amount = (0.055 + 0.15 * glint + 0.30 * band) * response
            * (0.35 + 0.45 * chroma + 0.20 * contour);
        light = tint * amount + sheen * edge_light * 0.32;
    } else if finish == 2u {
        // Engraved facets follow pigment boundaries, with a warm/cool metallic
        // reflection and antialiased hairlines rather than a flat silver wash.
        let phase = (uv.x - uv.y) * 240.0 + luma * 12.0;
        let grain = (0.5 + 0.5 * sin(phase))
            * (1.0 - smoothstep(0.7, 2.5, fwidth(phase)));
        let tilt = 0.5 + 0.5 * sin(time * 0.38 + uv.y * 5.0 + angle * 3.0);
        let metal = mix(vec3<f32>(0.40, 0.67, 0.82), vec3<f32>(0.96, 0.77, 0.43), tilt);
        let facet = pow(0.5 + 0.5 * sin(luma * 22.0 + uv.x * 8.0 - time * 0.7), 6.0);
        light = metal * response * (0.045 + 0.27 * band + 0.15 * facet)
            * (0.28 + 0.62 * contour + 0.22 * grain) + metal * edge_light * 0.65;
    } else if finish == 3u {
        // Curved diffraction rings bend through the actual pigment values.
        let radius = length((uv - vec2<f32>(0.35, 0.4)) * vec2<f32>(1.0, 1.4));
        let phase = radius * 3.4 + luma * 0.65 - time * 0.10 + angle;
        let spectrum = 0.5 + 0.5 * cos(6.283185 *
            (vec3<f32>(phase) + vec3<f32>(0.0, 0.3333, 0.6667)));
        light = spectrum * response * (0.10 + 0.32 * band) * (0.5 + chroma)
            + spectrum * edge_light * 0.5;
    } else if finish == 4u || finish == 5u {
        // Stable cells twinkle gradually: no frame-random noise or strobing.
        let density = select(115.0, 38.0, finish == 5u);
        let point = uv * vec2<f32>(density, density / CARD_ASPECT);
        let cell = floor(point);
        let seed = fract(sin(dot(cell, vec2<f32>(127.1, 311.7))) * 43758.5453);
        let offset = vec2<f32>(seed, fract(seed * 17.13)) * 0.6 + 0.2;
        let delta = abs(fract(point) - offset);
        let aa = max(length(fwidth(point)) * 0.4, 0.04);
        let dot_light = 1.0 - smoothstep(0.03, 0.12 + aa, length(delta));
        let twinkle = pow(0.5 + 0.5 * sin(time * 1.15 + seed * 31.0 + angle * 4.0), 8.0);
        let tint = mix(vec3<f32>(0.38, 0.70, 1.0), vec3<f32>(1.0, 0.74, 0.38), seed);
        var sparkle = dot_light * twinkle * smoothstep(0.36, 0.8, seed);
        if finish == 5u {
            let cross = exp(-delta.x * 65.0) * exp(-delta.y * 9.0)
                + exp(-delta.y * 65.0) * exp(-delta.x * 9.0);
            sparkle = (dot_light + cross * 0.65) * twinkle * smoothstep(0.70, 0.95, seed);
            let cloud = 0.5 + 0.5 * sin(uv.x * 8.0 + uv.y * 5.0 + luma * 6.0 + time * 0.22);
            light = mix(vec3<f32>(0.24, 0.12, 0.55), vec3<f32>(0.10, 0.45, 0.52), cloud)
                * response * band * 0.18;
        }
        light += tint * response * (sparkle * 0.85 + band * 0.055)
            + mix(vec3<f32>(0.35, 0.55, 0.90), vec3<f32>(0.90, 0.65, 0.35),
                0.5 + 0.5 * sin(uv.y * 4.0 + time * 0.4)) * edge_light * 0.40;
    }
    return vec4<f32>(printed.rgb + (vec3<f32>(1.0) - rgb) * light, printed.a);
}

/// How tight the travelling highlight is, as a Gaussian falloff.
///
/// Paired with [`SWEEP_MARGIN`]: at the margin the band is `exp(-6.5)`, which
/// is a thousandth of its peak, so it is genuinely off the card at both ends
/// of the travel and needs no separate fade to keep it from popping.
const SWEEP_WIDTH: f32 = 26.0;

/// How far past the card the band starts and finishes, in diagonal units.
const SWEEP_MARGIN: f32 = 0.5;

/// The one-shot sheen: a bright band crossing the card once, from its bottom
/// right corner to its top left.
///
/// # Why this replaced a loop
///
/// The coating used to sweep continuously on every card in the hand, in the
/// preview and in the stack panel — a six-second cycle nothing ever stopped.
/// Measured on a still table with nothing hovered, a 24×16 frame diff put
/// *all* of the change in the hand zone, mean 18–30 per cell against a table
/// at zero: the "flickering" a player sees at rest is this band, and a
/// glamour every card wears at all times is not a glamour. So it happens
/// once, when there is something to notice — a card drawn, a card played, a
/// preview opened — and then the card is one of the ones you already know.
///
/// There is no metal under it any more: the coating every card used to wear
/// lifted the print's blacks and went with #274. This is light travelling
/// across the card once, and it leaves nothing behind.
///
/// `phase` runs 0 → 1 over the sweep and is outside that range at every other
/// moment, so a card with no sweep passes any value below zero and gets
/// nothing. `uv.y` is 0 at the head of the card, so `(u+v)/2` is 1 at the
/// bottom right and 0 at the top left and the band is perpendicular to that
/// diagonal — which is the direction a card catches the light when it is
/// turned over towards you.
fn sweep_amount(uv: vec2<f32>, phase: f32) -> f32 {
    if phase < 0.0 || phase > 1.0 {
        return 0.0;
    }
    let along = (uv.x + uv.y) * 0.5;
    let line = mix(1.0 + SWEEP_MARGIN, -SWEEP_MARGIN, phase);
    let off = along - line;
    return exp(-off * off * SWEEP_WIDTH);
}

/// The five doors a permanent goes through, numbered as `cardmat::door`
/// numbers them. Zero is the plain arrival above, and is what almost every
/// sweep in a game is.
const DOOR_NONE: u32 = 0u;
const DOOR_BOUNCE: u32 = 1u;
const DOOR_EXILED: u32 = 2u;
const DOOR_FLICKERED: u32 = 3u;
const DOOR_DESTROYED: u32 = 4u;
const DOOR_RETURNED: u32 = 5u;

/// How tight a door's figure is, as a Gaussian falloff.
///
/// Looser than [`SWEEP_WIDTH`]'s 26 on purpose: the arrival band is a
/// highlight on a coating and wants to read as a specular line, while a door
/// is a thing *happening to* the card and wants some air around it.
///
/// It was 11, which is what the first screenshot of a forced door was for.
/// At 11 the falloff is a fifth of the card wide at half strength and the
/// band did not cross the card at all — it washed the whole of it amber, art
/// and text and border together, which reads as "this card is now orange"
/// rather than as anything travelling. 40 puts the band at about a tenth of
/// the card and leaves the face legible on both sides of it.
const DOOR_WIDTH: f32 = 40.0;

/// How far a ring travels before it is gone, in card widths from the middle.
///
/// The card's half-diagonal in these units is about 0.86, so 0.95 is just
/// past the corners: a closing ring spends its whole phase somewhere on the
/// card and shuts on the middle, rather than spending the first half of it
/// off the edge where nothing can be seen. It was 1.15, and the difference
/// is one screenshot: at 1.15 the ring only arrives on the card at the very
/// end and reads as a glow that appears, not as a portal that closes.
const DOOR_REACH: f32 = 0.95;

/// How strongly a door's colour is laid over the card.
///
/// It is an addition, like the sheen, so this is a peak and not a mix: the
/// art underneath keeps its colour and the door happens on top of it. Three
/// times the coating's `METAL_GLOSS` because a door is a *statement* — a
/// player who missed it has missed the only thing the client will ever say
/// about where that card went — and no more than that, because 0.85 (the
/// first guess) saturates every channel it touches and takes the card with
/// it.
const DOOR_GLOSS: f32 = 0.60;

/// A ring travelling out from the middle of the card, or in towards it.
///
/// This is the portal, and both of the exile doors are it: `radius` runs
/// outwards for a card flickering in and inwards for one being exiled, which
/// is the owner's "the same effect reversed" said in one argument.
fn door_ring(uv: vec2<f32>, radius: f32) -> f32 {
    // Width-units, so the ring is round on the card rather than round in UV
    // — a card is 88 tall for every 63 across, and a circle in UV is an egg.
    let p = (uv - vec2<f32>(0.5, 0.5)) * vec2<f32>(1.0, 1.0 / CARD_ASPECT);
    let off = length(p) - radius;
    // Twice as tight as the bands: a ring is a closed curve, so at the band's
    // own width it is a filled disc long before it reaches the middle.
    return exp(-off * off * DOOR_WIDTH * 2.0);
}

/// A band crossing the card along one axis, from `head` to `foot`.
///
/// The two graveyard doors are this: destruction falls down the card and a
/// resurrection rises up it, which is the same reversal the ring makes and
/// for the same reason — a player learns one figure and reads two events.
fn door_band(along: f32, phase: f32, downwards: bool) -> f32 {
    let travel = select(1.0 - phase, phase, downwards);
    let line = mix(-SWEEP_MARGIN, 1.0 + SWEEP_MARGIN, travel);
    let off = along - line;
    return exp(-off * off * DOOR_WIDTH);
}

/// The colour a door is drawn in.
///
/// Five colours, and the pairs are deliberately the *same* colour in both
/// directions: exile and its flicker are one violet, a destruction and a
/// resurrection are one amber. The figure says which way the card was going,
/// and asking the colour to say it as well would leave a player learning ten
/// marks instead of three.
///
/// Amber and not red for the graveyard, because red on a card is damage and
/// this client already draws damage. Violet for exile, because nothing else
/// on the table is violet. Pale blue for a bounce, because it is the
/// gentlest of the five — the card is going somewhere its owner still has
/// it.
fn door_tone(door: u32) -> vec3<f32> {
    if door == DOOR_BOUNCE {
        return vec3<f32>(0.36, 0.62, 0.92);
    }
    if door == DOOR_EXILED || door == DOOR_FLICKERED {
        return vec3<f32>(0.62, 0.34, 0.90);
    }
    if door == DOOR_DESTROYED || door == DOOR_RETURNED {
        return vec3<f32>(0.95, 0.63, 0.24);
    }
    return vec3<f32>(0.0);
}

/// What a door adds to the card, colour and all.
///
/// Returns black for [`DOOR_NONE`] and for a phase outside the one pass, so a
/// card that is not going anywhere pays one compare and gets nothing — which
/// is every card on the table almost all of the time.
fn door_layer(uv: vec2<f32>, phase: f32, door: u32) -> vec3<f32> {
    if door == DOOR_NONE || phase < 0.0 || phase > 1.0 {
        return vec3<f32>(0.0);
    }
    var figure = 0.0;
    if door == DOOR_BOUNCE {
        // The entrance played backwards, so it is the arrival's own band and
        // not a new figure — travelling the other way along the diagonal,
        // because the card is leaving by the way it came.
        let along = (uv.x + uv.y) * 0.5;
        let line = mix(-SWEEP_MARGIN, 1.0 + SWEEP_MARGIN, phase);
        let off = along - line;
        figure = exp(-off * off * DOOR_WIDTH);
    } else if door == DOOR_EXILED {
        figure = door_ring(uv, mix(DOOR_REACH, 0.0, phase));
    } else if door == DOOR_FLICKERED {
        figure = door_ring(uv, mix(0.0, DOOR_REACH, phase));
    } else if door == DOOR_DESTROYED {
        figure = door_band(uv.y, phase, true);
    } else if door == DOOR_RETURNED {
        figure = door_band(uv.y, phase, false);
    }
    return door_tone(door) * figure * DOOR_GLOSS;
}

/// How many keywords the strip can carry.
const MARK_COUNT: u32 = 15u;

// The strip's shape, in card widths: `baylee_client_core::cardrail`'s
// constants, mirrored, and `cardmat`'s tests read these lines to hold the two
// to the same digits. Where the strip lies on the card is not here at all:
// the quad it is drawn on is placed by the table and the preview, and this
// file only draws inside it.

/// The strip's inner margin round its marks. `cardrail::STRIP_PAD`.
const STRIP_PAD: f32 = 0.012;

/// A mark's square, eight physical pixels on a card 94 wide.
/// `cardrail::MARK`.
const MARK: f32 = 0.085;

/// The air between two marks. `cardrail::MARK_GAP`.
const MARK_GAP: f32 = 0.012;

/// How far the strip's contact shadow spreads past it, on the left, the right
/// and the top. The quad has no margin below: the strip stands on the seam's
/// rule, and under it is the type line. `cardrail::SHADOW_MARGIN`.
const SHADOW_MARGIN: f32 = 0.02;

/// How dark the contact shadow is where it meets the strip, as coverage of
/// black. It falls off as the square of the distance, so by half the margin
/// it is a quarter of this.
const SHADOW_DEPTH: f32 = 0.55;

/// The strip's corner radius.
const STRIP_CORNER: f32 = 0.018;

/// The strip's edge, a slate lighter than the plate, so a dark strip on dark
/// artwork still has an outline.
const STRIP_RIM: vec3<f32> = vec3<f32>(0.16, 0.17, 0.19);

/// The beat every mark breathes on, in radians per second.
///
/// One beat, phase-offset per slot: five marks that each animated on their
/// own timing would strobe, and thirty cards of them would be a fairground.
const BEAT: f32 = 1.15;

/// The strip's own colour, so ivory reads on any artwork.
const PLATE: vec3<f32> = vec3<f32>(0.045, 0.052, 0.062);

/// The ink every mark is drawn in, before its own colour is mixed into it.
const INK: vec3<f32> = vec3<f32>(0.94, 0.96, 0.99);

fn sd_box(p: vec2<f32>, b: vec2<f32>) -> f32 {
    let q = abs(p) - b;
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0);
}

fn sd_round_box(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    return sd_box(p, b - vec2<f32>(r)) - r;
}

// ---- the twelve marks
//
// Each one is a glyph of the Mana font, baked to a signed distance field at
// startup by `markatlas.rs` and sampled out of one atlas row. They used to be
// twelve procedural drawings here — a chevron, a shield, an eye — and the
// argument for replacing them is not that they were bad: it is that a player
// arriving at this table has already learned Magic's own ability icons
// somewhere else, and no drawing of ours can be the picture they already
// know. `docs/legal.md` §2a is where the licence and the trademark that come
// with that are kept apart.
//
// Cell coordinates still run -0.5..0.5 with **y downward**, the way the
// card's UV does, and the sampler clamps inside the cell so a mark can never
// fetch its neighbour's texels.
//
// # What a motion has to be, at ten pixels
//
// A card on the felt was 86 physical pixels wide when this was measured, and
// a rail slot was 0.115 of that, so **one cell was ten pixels**, and one cell
// unit was those ten pixels. That number is the whole of this design, and it
// was arrived at by photographing a table rather than by reading this file.
// Since #274 a mark is `MARK` of a card 94 pixels wide — eight pixels — so
// everything below about what a sub-pixel movement cannot show holds with a
// fifth less room.
//
// The rail once shipped with nine marks carrying a `sin(ph)` term of 0.012 to
// 0.03 cell units: between an eighth and a third of a pixel. Measured live,
// with the clock walked by `/step` and each cell photographed, a mark that
// ignores `ph` altogether swings 20 levels out of 255 — the
// `0.90 + 0.10 * sin(phase)` ink pulse, alone — and a 0.12-pixel sway swung
// 23. Three levels is what a third of a pixel buys. The eye that changed
// *shape* swung 147, and a pip that travelled 1.7 pixels swung 89.
//
// The mechanism is in the antialiasing. `e = max(aa / slot, 0.02)`, and
// `fwidth(p.x)` is 0.0116 width units on an 86-pixel card, so the ink ramp
// is exactly one pixel wide and a one-pixel stroke never reaches full ink at
// all. Displacing it by a third of a pixel moves no pixel and changes no
// total ink: it redistributes intensity between two of them, which is a
// second brightness pulse in the one channel the ink pulse already owns.
//
// So **rest is the row's state and motion is an event**, and an event has to
// turn pixels over by about a stroke's width. A sampled glyph cannot change
// shape, so the twelve verbs the drawings had — a flap, a blink, a ground
// opening — are gone with them, and what is left is the one whole-glyph verb
// that still turns pixels over: **scale**. A mark growing a fifth moves its
// whole silhouette outward by a tenth of a cell, a pixel on every side at
// once, which is the eye's old magnitude spent in a different place. Whole-
// glyph *translation* is not on this list and must not come back: it was
// measured at three levels out of 255 and it is invisible.
//
// # And rarely
//
// A strip of twelve symbols that all move is not a row of symbols. Every
// impulse below runs `mark_event` over `fract(ph * K)`, and `fract` has
// period **one**: with `ph` advancing at `BEAT` radians a second, a rate `K`
// wraps every `1 / (BEAT * K)` seconds, which is 0.8696 / K and not the
// 5.464 / K it would be if the term were a `sin`. Reading it as a `sin`
// makes every period on this list a factor of tau too slow, and that is the
// mistake this paragraph exists to stop.
//
// The rates are the ones the drawings were measured on, kept because the
// measurement was of the *rarity* and not of the shape:
//
//     haste     0.0896   9.7 s      reach        0.0521  16.7 s
//     trample   0.0731  11.9 s      deathtouch   0.0458  19.0 s
//     flying    0.0664  13.1 s      the strikes  0.0398  21.9 s
//     vigilance 0.0600  14.5 s      menace       0.0361  24.1 s
//
// with lifelink alone left on the beat, because a heart is the one keyword
// whose meaning *is* a period, and prowess on its own slow `sin` because a
// bonus with a deadline is the one claim that is about *this turn*. An event
// lasts one to two seconds, so a mark is still for nine tenths of its life
// and a full strip of twelve has, on average, nine tenths of one mark moving.

/// One mark's square in the atlas, in texels. `markatlas::CELL`.
const MARK_CELL: f32 = 96.0;

/// How far either side of the outline a *mark* cell encodes, in cell units.
/// `markatlas::RANGE`. The corner's text cells carry their own, shorter one.
const MARK_RANGE: f32 = 0.25;

/// How many cells the atlas holds altogether. `markatlas::CELLS`.
///
/// One row and one texture: the strip's twelve marks, then the corner's
/// fifteen characters. Only the bake rule and the encoded range differ
/// between the two halves, and both of those are settled before a texel is
/// written — so a second texture would be two more bindings in two
/// materials for nothing.
const ATLAS_CELLS: u32 = 34u;

/// The envelope every impulse on the strip shares: up over `a`, held until
/// `h`, down over `r`, and flat zero through the rest of the period — which
/// is most of it. All three are fractions of that mark's own period, so the
/// numbers at each call site are read against the table above.
fn mark_event(f: f32, a: f32, h: f32, r: f32) -> f32 {
    return smoothstep(0.0, a, f) - smoothstep(h, h + r, f);
}

/// The two strikes share a clock: one beat in four, landing a tenth of a
/// beat early.
///
/// A rate of `1 / (4 * 2 * pi)` is a whole beat every fourth one — the only
/// place on the strip where a period is *meant* to line up with the ink
/// pulse, because first strike's claim is about arriving before something,
/// and earliness is only legible against a reference. Do not expect a player
/// to feel it; it costs nothing and is right when lifelink is on the same
/// strip to be early *against*.
fn strike_clock(ph: f32) -> f32 {
    return fract(ph * 0.03979 - 0.0375);
}

/// How big a mark is drawn, as a multiple of its resting size.
///
/// One verb, twelve readings of it. The amplitudes are the same everywhere
/// but the strikes, because a strip whose marks swelled by different amounts
/// would be saying something about their importance; what differs is the
/// *envelope* — how sharply a mark arrives at its size and how long it holds
/// there — which is the difference between a blow, a breath and a heartbeat.
///
/// STILL: defender's *scale* does not move. A shield is the keyword for a
/// creature that does not act, and a mark that holds its size while its
/// neighbours swell says so without spending a second colour on it.
///
/// This arm is the only thing that sentence is true of, and the comment here
/// claimed the whole drawing until #102. Scale is not the mark's only
/// movement: the ink in `label_strip` breathes every mark on the strip —
/// `(0.95 + 0.05 * sin(phase))`, with no `which` in it — so defender has no
/// swell *event* and is in continuous motion exactly like everything beside
/// it. "It is the stillness that is the drawing" was the argument, and the
/// stillness is not there to be drawn.
///
/// Measured rather than argued, because the number cuts both ways: that
/// breath is **10.8 display levels** peak to peak on this mark's warm stone
/// over a 5.5 s cycle, against the **20** the rim measurement set as the
/// floor for a signal read across the table. So the claim is false *and* a
/// `which` guard here would buy a stillness no player could tell from this
/// one. That is why #102 was answered by correcting the claim instead of
/// changing the picture; it is evidence for #23 and not a decision taken
/// here, and #23 is where the design still sits.
fn mark_pulse(which: u32, ph: f32) -> f32 {
    switch which {
        // Flying: a long glide, then three quick beats of a wing.
        case 0u: {
            let ev = mark_event(fract(ph * 0.0664), 0.0115, 0.0878, 0.0267);
            return 1.0 + 0.16 * pow(0.5 + 0.5 * sin(ph * 11.0), 2.0) * ev;
        }
        // First strike: a blow. Nothing else on the strip arrives this fast.
        case 1u: { return 1.0 + 0.22 * mark_event(strike_clock(ph), 0.004, 0.030, 0.026); }
        // Double strike: the same blow, twice, on the same clock.
        case 2u: {
            let f = strike_clock(ph);
            return 1.0
                + 0.19 * mark_event(f, 0.004, 0.014, 0.010)
                + 0.19 * mark_event(f, 0.026, 0.036, 0.026);
        }
        // Deathtouch: a swell that arrives slowly and does not let go.
        case 3u: { return 1.0 + 0.17 * mark_event(fract(ph * 0.0458), 0.0380, 0.0700, 0.0420); }
        // Haste: there and gone.
        case 4u: { return 1.0 + 0.20 * mark_event(fract(ph * 0.0896 + 0.11), 0.0090, 0.0330, 0.0180); }
        // Lifelink: the beat itself, and the only mark with no envelope —
        // a heart that beat rarely would be a heart in trouble.
        case 5u: { return 1.0 + 0.09 * pow(0.5 + 0.5 * sin(ph * 1.7), 3.0); }
        // Menace: it looms, holds, and withdraws.
        case 6u: { return 1.0 + 0.18 * mark_event(fract(ph * 0.0361 + 0.63), 0.0332, 0.0623, 0.0228); }
        // Reach: a slow climb to full height.
        case 7u: { return 1.0 + 0.17 * mark_event(fract(ph * 0.0521 + 0.79), 0.0330, 0.0620, 0.0240); }
        // Trample: it lands, and the landing is the whole of it.
        case 8u: { return 1.0 + 0.19 * mark_event(fract(ph * 0.0731 + 0.47), 0.0070, 0.0300, 0.0290); }
        // Vigilance: the calibration standard, a fifth of a second wider
        // than it was — the eye used to close and a shape has to travel
        // further than a size to say the same thing.
        case 9u: { return 1.0 + 0.17 * mark_event(fract(ph * 0.0600), 0.0250, 0.0560, 0.0280); }
        // Defender: STILL — the scale only. The ink in `label_strip` breathes
        // this mark with the rest of the row; see the note on this function
        // and `the_ink_below_a_mark_is_not_told_which_mark_it_is`.
        case 10u: { return 1.0; }
        // Prowess: rises, hangs, and comes down — a bonus with a deadline.
        case 11u: { return 1.0 + 0.15 * pow(0.5 + 0.5 * sin(ph * 0.85), 5.0); }
        // Hexproof: STILL. A protection is a state the card is in and not
        // an event it does; the shell round the card is the glance, and the
        // mark is the exact statement (#298).
        case 12u: { return 1.0; }
        // Indestructible: STILL, for hexproof's reason.
        case 13u: { return 1.0; }
        // Shroud: STILL, for hexproof's reason.
        case 14u: { return 1.0; }
        default: { return 1.0; }
    }
}

/// The distance field in one cell of the baked atlas, read at `p`.
///
/// `marks` is one row of `ATLAS_CELLS` squares, each holding a distance with
/// the outline at 0.5 and `range` cell units of field either side of it —
/// the strip's marks, the corner's alphabet and the identity column's three
/// symbols, which differ in how they were baked and in nothing else. This is
/// the only function that fetches from it, which is why the two awkward
/// parts of that fetch are written down once:
///
/// The fetch is **clamped half a texel inside the cell**, because a linear
/// sample straddles two texels and the one past the wall belongs to the next
/// cell along. What the clamp moves is added back to the distance: a point
/// dragged in from outside the cell is at least that far from anything drawn
/// in it, so the halo keeps falling off past the wall instead of stopping
/// flat against it.
///
/// And it is `textureSampleLevel` and not `textureSample`: a cell is picked
/// in *non-uniform* control flow — which slot a fragment lands in is the
/// whole point of the strip — and an implicit derivative asked for there is
/// undefined. There is nothing to choose a mip from in any case; the atlas
/// has one level.
fn cell_sdf(
    cell: u32,
    p: vec2<f32>,
    range: f32,
    marks: texture_2d<f32>,
    marks_s: sampler,
) -> f32 {
    let wanted = p + vec2<f32>(0.5);
    let inset = 0.5 / MARK_CELL;
    let inside = clamp(wanted, vec2<f32>(inset), vec2<f32>(1.0 - inset));
    let uv = vec2<f32>((f32(cell) + inside.x) / f32(ATLAS_CELLS), inside.y);
    let field = textureSampleLevel(marks, marks_s, uv, 0.0).r;
    return (0.5 - field) * (2.0 * range) + length(wanted - inside);
}

/// The strip's own reading of a cell: the same fetch, with the pulse on it.
///
/// The **pulse divides** rather than multiplying, because growing a picture
/// means reading its field closer to the middle, and the distance that comes
/// back is in the grown glyph's units and has to be scaled out again.
fn mark_sdf(which: u32, p: vec2<f32>, ph: f32, marks: texture_2d<f32>, marks_s: sampler) -> f32 {
    if which >= MARK_COUNT {
        return 1.0;
    }
    let grown = mark_pulse(which, ph);
    return cell_sdf(which, p / grown, MARK_RANGE, marks, marks_s) * grown;
}

/// Each mark's own colour, used for its halo and mixed into its ink.
///
/// Bright enough to survive the plate, and far enough apart that the row
/// still separates once the marks are too small to read as drawings — which
/// is the honest failure mode: at board scale twelve keywords are twelve
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
        // Warm stone: flying, first strike and vigilance are all pale
        // blue-white, and at table scale, where the mark is a pip, four
        // pips of one colour are one pip. A tower is the member of that
        // group with a colour of its own to take.
        case 10u: { return vec3<f32>(0.82, 0.76, 0.66); }
        case 11u: { return vec3<f32>(0.36, 0.90, 0.86); }
        // Hexproof blue and shroud lavender, the shells' own colours (#298),
        // and indestructible the grey of steel.
        case 12u: { return vec3<f32>(0.40, 0.66, 1.00); }
        case 13u: { return vec3<f32>(0.66, 0.70, 0.76); }
        case 14u: { return vec3<f32>(0.84, 0.66, 1.00); }
        default: { return INK; }
    }
}

/// The label's bits: the sleep moon, and two identity crests after it, two
/// bits each, `glyph + 1` or zero for none. `cardrail::label`.
const LABEL_MOON: u32 = 1u;
const LABEL_CREST_SHIFT: u32 = 1u;
const LABEL_CREST_BITS: u32 = 2u;

/// How many things a strip can carry: the chip, the moon, every mark and
/// two crests, in the order they are packed from the left.
const LABEL_ITEMS: u32 = 19u;

/// How long a row runs inside the strip's margin before a cell opens a row
/// above it. `cardrail::ROW_MAX`.
const ROW_MAX: f32 = 0.70;

/// How far the strip's shadow is thrown up the card past the sides', in
/// card widths: the lamp stands at the player's side of the table, so a
/// thing lying *on* the card throws its shadow away from them, over the art.
/// None falls below the strip, where the type line is.
const SHADOW_THROW: f32 = 0.006;

/// The strip's lit upper edge and shaded lower one: what makes a flat quad
/// read as a slab lying on the card (#298) rather than paint on it.
const STRIP_LIT: vec3<f32> = vec3<f32>(0.34, 0.36, 0.40);
const STRIP_LIP: vec3<f32> = vec3<f32>(0.010, 0.011, 0.014);

/// Item `i`'s size, `(width, height)` in card widths, or zero for one this
/// strip does not carry. `cardrail::Strip`'s items, in their order.
fn label_item(i: u32, bits: u32, swing: u32, label: u32) -> vec2<f32> {
    if i == 0u {
        return select(vec2<f32>(0.0), vec2<f32>(CHIP_W, PLATE_H), (swing & SWING_SET) != 0u);
    }
    if i == 1u {
        return select(vec2<f32>(0.0), vec2<f32>(MARK), (label & LABEL_MOON) != 0u);
    }
    if i < 2u + MARK_COUNT {
        return select(vec2<f32>(0.0), vec2<f32>(MARK), (bits & (1u << (i - 2u))) != 0u);
    }
    let n = i - 2u - MARK_COUNT;
    let field = (label >> (LABEL_CREST_SHIFT + LABEL_CREST_BITS * n)) & 3u;
    return select(vec2<f32>(0.0), vec2<f32>(MARK), field != 0u);
}

/// The sleep moon in its cell: a crescent, two discs one bitten out of the
/// other, in `cell` units, -0.5..0.5.
fn moon_sdf(cell: vec2<f32>) -> f32 {
    let lit = length(cell - vec2<f32>(-0.04, 0.0)) - 0.36;
    let bite = length(cell - vec2<f32>(0.14, -0.10)) - 0.30;
    return max(lit, -bite);
}

/// The keyword strip, and since #298 the card's label: the chip, the sleep
/// moon, the marks and the identity crests, packed from the
/// left along the seam, a cell that would run past `ROW_MAX` opening a row
/// above (`cardrail::Strip::cells`, which is the same arithmetic in Rust).
///
/// `q` is the point in the quad, card widths from its top-left corner, and
/// `quad` its size; the strip's bottom is the quad's, on the seam, and its
/// left end is `SHADOW_MARGIN` in. Every word is a uniform, so the layout
/// loop runs in uniform control flow and only the one cell under `q` is
/// drawn.
fn label_strip(
    q: vec2<f32>,
    quad: vec2<f32>,
    bits: u32,
    swing: u32,
    label: u32,
    t: f32,
    aa: f32,
    marks: texture_2d<f32>,
    marks_s: sampler,
) -> vec4<f32> {
    let first = select(MARK, PLATE_H, (swing & SWING_SET) != 0u);
    let x0 = SHADOW_MARGIN;
    let seam = quad.y - STRIP_PAD;

    // Lay every item out, as `cardrail::Strip::layout` does, and keep the
    // one whose cell — with half the gap round it — holds this point.
    var x = 0.0;
    var row = 0u;
    var width = 0.0;
    var top = seam;
    var count = 0u;
    var hit = LABEL_ITEMS;
    var hit_rect = vec4<f32>(0.0);
    var hit_k = 0u;
    for (var i = 0u; i < LABEL_ITEMS; i = i + 1u) {
        let size = label_item(i, bits, swing, label);
        if size.x <= 0.0 {
            continue;
        }
        if x > 0.0 && x + size.x > ROW_MAX {
            row = row + 1u;
            x = 0.0;
        }
        var bottom = seam;
        var tall = first;
        if row > 0u {
            bottom = seam - (first + MARK_GAP + f32(row - 1u) * (MARK + MARK_GAP));
            tall = MARK;
        }
        let mid_y = bottom - tall * 0.5;
        let left = x0 + STRIP_PAD + x;
        width = max(width, x + size.x);
        top = min(top, bottom - tall);
        let band = q.y >= bottom - tall - 0.5 * MARK_GAP && q.y < bottom + 0.5 * MARK_GAP;
        if band && q.x >= left - 0.5 * MARK_GAP && q.x < left + size.x + 0.5 * MARK_GAP {
            hit = i;
            hit_rect = vec4<f32>(left, mid_y - 0.5 * size.y, left + size.x, mid_y + 0.5 * size.y);
            hit_k = count;
        }
        count = count + 1u;
        x = x + size.x + MARK_GAP;
    }
    if count == 0u {
        return vec4<f32>(0.0);
    }

    // The slab: `cardrail::Strip::rect`.
    let size = vec2<f32>(2.0 * STRIP_PAD + width, quad.y - (top - STRIP_PAD));
    let mid = vec2<f32>(x0 + 0.5 * size.x, quad.y - 0.5 * size.y);
    let half = 0.5 * size;
    let body = sd_round_box(q - mid, half, STRIP_CORNER);
    let cover = 1.0 - smoothstep(-aa, aa, body);

    // Its shadow falls on the art round it, thrown a little further up the
    // card than to the sides, and is gone by the quad's edge; below the
    // strip there is no quad to fall on.
    let thrown = sd_round_box(q - mid + vec2<f32>(0.0, SHADOW_THROW), half, STRIP_CORNER);
    let fall = 1.0 - smoothstep(0.0, SHADOW_MARGIN, max(thrown, 0.0));
    let shade = SHADOW_DEPTH * fall * fall;

    // The slab's edge, lit along the top and in shade along the bottom, and
    // a rim a pixel and a half wide between, so the strip keeps an outline
    // on artwork as dark as it is.
    let edge = 1.0 - smoothstep(0.0, 1.5 * aa, -body);
    let upper = smoothstep(-0.25 * size.y, 0.25 * size.y, mid.y - q.y);
    var out = mix(PLATE, mix(STRIP_LIP, STRIP_LIT, upper), edge);
    out = mix(out, STRIP_RIM, edge * 0.35);

    if hit == LABEL_ITEMS {
        return strip_over_shadow(out, cover, shade);
    }
    let centre = vec2<f32>(0.5 * (hit_rect.x + hit_rect.z), 0.5 * (hit_rect.y + hit_rect.w));
    let e = max(0.5 * aa / MARK, 0.012);

    // The chip.
    if hit == 0u {
        out = chip_layer(q, hit_rect.x, centre.y, swing, aa, out, marks, marks_s);
        return strip_over_shadow(out, cover, shade);
    }

    let cell = (q - centre) / MARK;
    if abs(cell.x) > 0.5 || abs(cell.y) > 0.5 {
        return strip_over_shadow(out, cover, shade);
    }

    // The moon: the plate's night ink, which a sleeping creature's numbers
    // are written in too.
    if hit == 1u {
        out = mix(out, MOON_INK, 1.0 - smoothstep(-e, e, moon_sdf(cell)));
        return strip_over_shadow(out, cover, shade);
    }

    // A crest.
    if hit >= 2u + MARK_COUNT {
        let n = hit - 2u - MARK_COUNT;
        let which = ((label >> (LABEL_CREST_SHIFT + LABEL_CREST_BITS * n)) & 3u) - 1u;
        out = crest_cell(cell, which, e, out, marks, marks_s);
        return strip_over_shadow(out, cover, shade);
    }

    // A mark.
    let which = hit - 2u;
    let phase = t * BEAT + f32(hit_k) * 0.22;
    let d = mark_sdf(which, cell, phase, marks, marks_s);
    let accent = mark_color(which);
    // 0.70 and not 0.55, because the halo no longer carries the colour. At
    // table scale a rim two texels wide is sub-pixel and the mark degrades to
    // a coloured pip — which is the design's own honest failure mode, and it
    // now has to happen in the ink or not at all.
    //
    // The pulse is 0.05 and was 0.10: it is the one *continuous* movement on a
    // strip whose whole argument is that rest is the state and motion is an
    // event, and a solid silhouette breathes over five times the pixels a
    // stroke did. Not removed, because `strike_clock` is timed to land early
    // against this beat and needs a beat to be early against.
    //
    // It is also not *guarded*, and that is the whole of #102: `which` is
    // read twice above this line and never again, so the twelve marks are
    // drawn by one arithmetic and defender's `mark_pulse` arm — the only
    // STILL one — stills its size and nothing else. `phase` offsets by `k`,
    // the mark's place, so the strip is a staggered breath rather than one;
    // #24's lane wave rides this same term and inherits the same answer,
    // which is why that ticket waited on this one. `cardmat`'s
    // `the_ink_below_a_mark_is_not_told_which_mark_it_is` pins the count, so
    // a guard added here cannot land without saying it is answering #23.
    let ink = mix(INK, accent, 0.70) * (0.95 + 0.05 * sin(phase));

    // Cell units, so the edge is as soft on a card in the preview as on one
    // across the table.
    //
    // Half of `aa`, and both numbers were tuned for a one-pixel stroke that
    // should never quite reach full ink. `aa / slot` was 0.101 cell at the
    // table, so the ramp spanned two whole pixels — on a solid glyph that is
    // not softness, it is a blur, and it is what closed the skull's eye
    // sockets and the tower's crenellations at 17 pixels. `e` above.
    // A rim, not a bloom. At 9 the halo was still 18% of the accent a tenth of
    // a cell out and 5% at the cell wall, so it filled every hole in a solid
    // silhouette — measured on deathtouch, whose eye sockets came back
    // *brighter* than its own ink — and painted each cell its own colour,
    // which put a visible seam between neighbouring marks.
    let halo = exp(-max(d, 0.0) * 24.0) * 0.30;
    out = out + accent * halo;
    out = mix(out, ink, 1.0 - smoothstep(-e, e, d));
    return strip_over_shadow(out, cover, shade);
}

/// The strip laid over its own shadow: `cover` of `plate`, and under the
/// rest of it `shade` of black, as one colour and one coverage for the
/// blend. The colour is not premultiplied; the blend multiplies it.
fn strip_over_shadow(plate: vec3<f32>, cover: f32, shade: f32) -> vec4<f32> {
    let alpha = cover + shade * (1.0 - cover);
    if alpha <= 0.0 {
        return vec4<f32>(0.0);
    }
    return vec4<f32>(plate * (cover / alpha), alpha);
}

// ------------------------------------------------------------ the identities
//
// Two questions about what a permanent *is*: is it a commander (CR 903.3),
// and is the card it looks like its own (CR 111.1, CR 707.2)? They were paper
// tabs under the printed name, which is on the print, then the frame's own
// paper (#274); since #298 each is a square of that paper at the strip's
// right end with its glyph printed on it. `baylee_client_core::cardcrest`
// is the Rust half.

/// Where the identity glyphs sit in the atlas.
const CREST_BASE: u32 = 31u;

/// Which glyph is which: `cardcrest::GLYPH_TOKEN`, `GLYPH_COPY` and
/// `GLYPH_COMMANDER`.
const CREST_TOKEN: u32 = 0u;
const CREST_COPY: u32 = 1u;
const CREST_COMMANDER: u32 = 2u;

/// The paper a token, a copy and a commander are printed on, by glyph.
/// `cardcrest::PAPER`.
///
/// Linear, and card stock rather than writing paper: displayed around 175 of
/// 255, dark enough to hold ink. Verdigris is a thing conjured rather than
/// printed; violet is what the swing uses for a permanent that is not what
/// it was; oxblood is a commander's.
const PAPER_TOKEN: vec3<f32> = vec3<f32>(0.396, 0.440, 0.418);
const PAPER_COPY: vec3<f32> = vec3<f32>(0.429, 0.385, 0.506);
const PAPER_COMMANDER: vec3<f32> = vec3<f32>(0.44, 0.17, 0.15);

/// The crests' ink, near-black so it holds on all three papers.
/// `cardcrest::CREST_INK`.
const CREST_INK: vec3<f32> = vec3<f32>(0.008, 0.007, 0.006);

/// One crest in its cell: its paper, and its glyph printed on it. `cell` is
/// the point in the cell's own units, -0.5..0.5 on both axes; `e` the edge's
/// softness in the same units. The paper is what reads at table size, the
/// glyph is the preview's caption — a legibility ladder, not the same claim
/// twice.
fn crest_cell(
    cell: vec2<f32>,
    which: u32,
    e: f32,
    under: vec3<f32>,
    marks: texture_2d<f32>,
    marks_s: sampler,
) -> vec3<f32> {
    var paper = PAPER_TOKEN;
    if which == CREST_COPY {
        paper = PAPER_COPY;
    } else if which == CREST_COMMANDER {
        paper = PAPER_COMMANDER;
    }
    let square = sd_round_box(cell, vec2<f32>(0.5), 0.18);
    var out = mix(under, paper, 1.0 - smoothstep(-e, e, square));
    let d = cell_sdf(CREST_BASE + which, cell, MARK_RANGE, marks, marks_s);
    out = mix(out, CREST_INK, 1.0 - smoothstep(-e, e, d));
    return out;
}

// ------------------------------------------------------------------ the plate
//
// A creature's power and toughness with the damage marked on it, or a
// planeswalker's loyalty, as an object of its own at the card's bottom right
// (the owner, 25.09), and the chip on the keyword strip. The Rust half is
// `baylee_client_core::cardplate`, which is where the numbers are packed and
// where every constant below is mirrored and tested.

/// How wide the plate is, its inner margin and its figure height, in card
/// widths. `cardplate::PLATE_W`, `PLATE_PAD`, `PLATE_CAP`.
const PLATE_W: f32 = 0.196;
const PLATE_PAD: f32 = 0.012;
const PLATE_CAP: f32 = 0.075;
/// The plate's height: its figures and its margin. `cardplate::PLATE_H`.
const PLATE_H: f32 = 0.099;

/// The air between the plate and the chip, and the chip's width.
/// `cardplate::CHIP_GAP` and `CHIP_W`.
const CHIP_GAP: f32 = 0.010;
const CHIP_W: f32 = 0.100;

/// How the packed word is read: three ten-bit numbers, two kind bits on top.
const PLATE_KIND_SHIFT: u32 = 30u;
const PLATE_SLOT_BITS: u32 = 10u;
const PLATE_SLOT_MASK: u32 = 0x3ffu;
const PLATE_BIAS: i32 = 128;

const PLATE_NONE: u32 = 0u;
const PLATE_FIGHT: u32 = 1u;
const PLATE_LOYALTY: u32 = 2u;
const PLATE_LORE: u32 = 3u;

/// The corner's alphabet: which cell of the atlas each character is, and
/// what it advances by.
///
/// A row of 4×6 bitmap stencils stood here, sampled bilinearly. Two
/// complaints came off it and they were one fault: it looked **blurry**,
/// because a mask that coarse smoothed up to eleven physical pixels is a
/// blur with no edge to sharpen, and it looked **off-centre**, because every
/// glyph was given the same four cells and the ink sat wherever the picture
/// put it — `1` in the left two, `/` corner to corner. A distance field has
/// an edge at any size, and an *advance* is what centring a line of type
/// means. The face is `AlegreyaSans-Bold`, which this client already sets
/// its interface in; `baylee_client_core::cardplate::TEXT_CHARS` is the
/// order and `markatlas` bakes them into the same row as the strip's marks.
const TEXT_BASE: u32 = 15u;
const TEXT_COUNT: u32 = 16u;
const GLYPH_MINUS: u32 = 10u;
const GLYPH_SLASH: u32 = 11u;
const GLYPH_PLUS: u32 = 12u;
const GLYPH_I: u32 = 13u;
const GLYPH_V: u32 = 14u;
const GLYPH_TIMES: u32 = 15u;

/// How far either side of an outline a text cell's field reaches, and how
/// tall a lining figure is — both in cell units, both mirrored.
const TEXT_RANGE: f32 = 0.10;
const TEXT_CAP: f32 = 0.57665;

/// The advances, in cell units. Seven numbers and not sixteen: the digits are
/// **tabular**, which is the difference between a creature growing from
/// `9/9` to `10/10` and one that also shunts its own slash sideways.
const TEXT_ADV_DIGIT: f32 = 0.45125;
const TEXT_ADV_MINUS: f32 = 0.28880;
const TEXT_ADV_SLASH: f32 = 0.26980;
const TEXT_ADV_PLUS: f32 = 0.45600;
const TEXT_ADV_I: f32 = 0.28025;
const TEXT_ADV_V: f32 = 0.55765;
const TEXT_ADV_TIMES: f32 = 0.45600;

/// The longest line the corner writes.
///
/// Eight, because nine four-bit indices do not fit in a word. It reaches
/// every number the plate can hold with a sign on one half — `-128/128` and
/// `895/895` are both eight — and falls one short of a sign on *both*, which
/// is a creature with a negative toughness and so a creature that a
/// state-based action removed before anybody was asked a question
/// (CR 704.5f). A line past this is truncated rather than wrapped, which is
/// the same choice `cardplate::slot` makes and for the same reason.
const TEXT_MAX: u32 = 8u;

/// The largest chapter written in roman numerals; a sixth falls back to
/// arabic, because `VI` needs a rule this composition does not have.
const ROMAN_MAX: u32 = 5u;

/// A planeswalker's rim and ink. Gilt, and explicitly not a shield: the
/// shield-shaped loyalty box is the printed planeswalker frame's own element,
/// and "a plain shield nobody owns" is the argument every borrowed frame
/// element makes (`docs/legal.md` §2).
const GILT: vec3<f32> = vec3<f32>(0.87, 0.73, 0.38);

/// Marked damage, which is the one thing on this plate that is not printed on
/// a real card — so it is drawn as a band rather than as a numeral.
///
/// Hot enough to be the **figure**. It was `(0.88, 0.27, 0.18)` laid on at
/// 58%, which composited to about `(138, 49, 36)` over the near-black body:
/// dark red behind white numerals, on a plate that is eighteen physical
/// pixels wide at the table camera. The owner played whole games without
/// noticing it, which is the correct reading of that picture — the numerals
/// were the figure and the damage was ground behind them.
///
/// So the band is laid on whole, and the numerals standing in it are drawn
/// in the plate's own colour instead (see [`plate_layer`]). A damaged
/// creature is a hot box with dark digits in it; an undamaged one is a dark
/// box with white ones. The hue sits at about 18°, well red of `GILT`'s 43°,
/// so a damaged creature is never read as a planeswalker.
const HEAT: vec3<f32> = vec3<f32>(0.96, 0.36, 0.18);

/// The shortest band any damage at all is drawn as, in card widths.
///
/// One point of damage on a twelve-toughness body is a hundredth of the
/// plate, which at this camera is a third of a pixel and says nothing. The
/// floor is about two and a half physical pixels at the table, which is
/// enough to read as *marked* — and the preview is where how much is asked.
const DAMAGE_FLOOR: f32 = 0.026;

/// Where the band stops being a proportion and starts being countable: the
/// pixel size, in card widths, at which it is ruled into one row per point
/// of toughness.
///
/// `aa` is that pixel size and is already computed, so the plate knows how
/// big it is being drawn without anything having to tell it — about 0.0106
/// on the table and 0.0018 in the hover preview. Rows at the table camera
/// would be a quarter of a pixel apart; at a quarter of this threshold they
/// are five pixels apart and a player counts them.
const TICK_AA: f32 = 0.004;

/// The largest toughness ruled into rows. Past it the rows are hairlines on
/// top of hairlines and the proportion reads better on its own.
const TICK_MAX: i32 = 12;

/// A rule's half-width, in pixels — a hairline by construction, at any size.
const TICK_HAIR: f32 = 0.6;

/// A saga's page. Light where every other plate is dark, because a chapter is
/// a *page* — and because the one thing that must never happen in this corner
/// is a chapter read as a power.
const PARCHMENT: vec3<f32> = vec3<f32>(0.88, 0.83, 0.69);
const SEPIA: vec3<f32> = vec3<f32>(0.24, 0.17, 0.10);

/// The swing's two words, as `cardplate::Corner::packed` writes them.
const SWING_SET: u32 = 0x100000u;
const TONE_SHIFT: u32 = 21u;
const TONE_PLAIN: u32 = 0u;
const TONE_DEADLY: u32 = 1u;
const TONE_TOXIC: u32 = 2u;
/// The plate's ink word's night bit. `cardplate::PLATE_NIGHT`.
const PLATE_NIGHT: u32 = 1u;
/// And its turned bit, `cardplate::PLATE_TURNED`: the face a half turn round.
const PLATE_TURNED: u32 = 2u;

/// A sleeping creature's plate ink, and its moon: moon-grey, still about
/// 5.8:1 on the plate's body.
const MOON_INK: vec3<f32> = vec3<f32>(0.50, 0.54, 0.64);


/// Deathtouch green, on the power alone.
///
/// The power and not both numbers, because that is the half of the body the
/// keyword acts through: a 1/1 deathtoucher trades with anything, and what
/// does the trading is the 1 on the left. Green rather than a thirteenth
/// mark on a strip that holds twelve — the number *is* the thing the keyword
/// changes the meaning of, so the colour belongs on it.
const DEADLY: vec3<f32> = vec3<f32>(0.42, 0.86, 0.45);

/// Toxic red, on both numbers. Nothing constructs it yet; see
/// `cardplate::Tone::Toxic`.
const TOXIC: vec3<f32> = vec3<f32>(0.95, 0.35, 0.33);

/// A swing that grew the creature, and one that shrank it.
///
/// The two colours the counter chips carried before the numerals took their
/// place — growth green and bruise violet — kept because they were the one
/// part of a chip that was read at a glance, and since #274 they are the
/// chip's own stock: at the nine pixels a chip is wide on the table the
/// colour is the reading, and the figures on it are the preview's.
const GROWN: vec3<f32> = vec3<f32>(0.40, 0.82, 0.46);
const SHRUNK: vec3<f32> = vec3<f32>(0.72, 0.46, 0.84);

/// How many glyph cells a chapter takes in roman numerals, for 1..=5.
fn roman_len(v: u32) -> u32 {
    if v == 4u { return 2u; }
    if v == 5u { return 1u; }
    return v;
}

/// The `k`-th roman glyph of a chapter: `I`, `II`, `III`, `IV`, `V`.
fn roman_at(v: u32, k: u32) -> u32 {
    if v == 5u { return GLYPH_V; }
    if v == 4u { return select(GLYPH_V, GLYPH_I, k == 0u); }
    return GLYPH_I;
}

/// One character's advance, in cell units.
fn text_adv(which: u32) -> f32 {
    if which < 10u { return TEXT_ADV_DIGIT; }
    switch which {
        case 10u: { return TEXT_ADV_MINUS; }
        case 11u: { return TEXT_ADV_SLASH; }
        case 12u: { return TEXT_ADV_PLUS; }
        case 13u: { return TEXT_ADV_I; }
        case 14u: { return TEXT_ADV_V; }
        default: { return TEXT_ADV_TIMES; }
    }
}

/// The signed distance to one character's outline, in cell units.
///
/// The same sampling as [`mark_sdf`] out of the same row, with two
/// differences and no third: the cell is offset past the marks, and the
/// field is decoded through the shorter [`TEXT_RANGE`] it was encoded with.
/// The clamp and the `length` that follows it are the same correction — a
/// point outside the cell is pulled to the wall, sampled there, and given
/// back the distance it was moved, so a glyph never fetches its
/// neighbour's texels and never reports a distance that stops at the wall.
fn text_sdf(which: u32, p: vec2<f32>, marks: texture_2d<f32>, marks_s: sampler) -> f32 {
    return cell_sdf(
        TEXT_BASE + min(which, TEXT_COUNT - 1u),
        p,
        TEXT_RANGE,
        marks,
        marks_s,
    );
}

/// A line of type, packed four bits a character with the count beside it.
///
/// `x` holds up to [`TEXT_MAX`] indices into the atlas's text half, least
/// significant first; `y` is how many. Two numbers rather than a string
/// because WGSL has neither, and four bits rather than five because the
/// alphabet is sixteen characters long — exactly what four bits hold, so a
/// seventeenth needs a wider packing — and eight of them have to fit in a
/// word.
fn text_push(line: vec2<u32>, which: u32) -> vec2<u32> {
    if line.y >= TEXT_MAX {
        return line;
    }
    return vec2<u32>(line.x | (which << (line.y * 4u)), line.y + 1u);
}

/// How many decimal digits a number is written in. Three is the ceiling a
/// ten-bit slot allows, so this needs no fourth case.
fn digits_of(v: u32) -> u32 {
    if v >= 100u { return 3u; }
    if v >= 10u { return 2u; }
    return 1u;
}

/// The `i`-th digit of `v` from the left, given it is written in `n` of them.
fn digit_at(v: u32, n: u32, i: u32) -> u32 {
    var p = 1u;
    for (var k = 0u; k < 2u; k = k + 1u) {
        if k + i + 1u < n {
            p = p * 10u;
        }
    }
    return (v / p) % 10u;
}

/// Appends a number to a line.
fn text_number(line: vec2<u32>, v: u32) -> vec2<u32> {
    var out = line;
    let n = digits_of(v);
    for (var i = 0u; i < 3u; i = i + 1u) {
        if i < n {
            out = text_push(out, digit_at(v, n, i));
        }
    }
    return out;
}

/// Appends a signed number, with its sign written out.
fn text_signed(line: vec2<u32>, v: i32, plus: bool) -> vec2<u32> {
    var out = line;
    if v < 0 {
        out = text_push(out, GLYPH_MINUS);
    } else if plus {
        out = text_push(out, GLYPH_PLUS);
    }
    return text_number(out, u32(abs(v)));
}

/// How wide a line is, in cell units.
fn text_width(line: vec2<u32>) -> f32 {
    var w = 0.0;
    for (var i = 0u; i < TEXT_MAX; i = i + 1u) {
        if i < line.y {
            w = w + text_adv((line.x >> (i * 4u)) & 0xfu);
        }
    }
    return w;
}

/// Where a line of type covers, and which of its characters is covering.
///
/// Returns the coverage in `x` and the character's ordinal in `y`, so that a
/// caller can ink one half of a line differently from the other without
/// laying the line out twice. `cap` is how tall its lining figures should
/// be, in card widths, and `mid` is the centre of the whole line.
///
/// The pen walk is a bounded loop over [`TEXT_MAX`] with the advances added
/// in order, which is what makes this typesetting rather than a grid: a `1`
/// takes the same box as an `8` because the digits are tabular, and a `/`
/// takes two thirds of one because the font says so.
fn text_cover(
    p: vec2<f32>,
    mid: vec2<f32>,
    cap: f32,
    line: vec2<u32>,
    marks: texture_2d<f32>,
    marks_s: sampler,
    aa: f32,
) -> vec2<f32> {
    if line.y == 0u {
        return vec2<f32>(0.0, 0.0);
    }
    // Cell units per card width, and the point in them, measured from the
    // line's left edge and from the middle of its figures.
    let unit = cap / TEXT_CAP;
    let width = text_width(line);
    let local = (p - mid) / unit + vec2<f32>(width * 0.5, 0.0);
    if local.x < 0.0 || local.x > width || abs(local.y) > 0.5 {
        return vec2<f32>(0.0, 0.0);
    }
    let e = max(aa / unit, 0.02);
    var pen = 0.0;
    var out = vec2<f32>(0.0, 0.0);
    for (var i = 0u; i < TEXT_MAX; i = i + 1u) {
        if i < line.y {
            let which = (line.x >> (i * 4u)) & 0xfu;
            let adv = text_adv(which);
            if local.x >= pen && local.x < pen + adv {
                let q = vec2<f32>(local.x - pen - adv * 0.5, local.y);
                let d = text_sdf(which, q, marks, marks_s);
                out = vec2<f32>(1.0 - smoothstep(-e, e, d), f32(i));
            }
            pen = pen + adv;
        }
    }
    return out;
}

/// The figure height a line may have, given the height it wants and the
/// width it has to fit.
///
/// Shrinking rather than clipping is the degradation that stays honest: a
/// plate that cut a digit off would be showing a number that is wrong,
/// while a `10/10` a size smaller than a `2/2` is still a `10/10`.
fn fit(cap: f32, line: vec2<u32>, room: f32) -> f32 {
    return min(cap, room * TEXT_CAP / max(text_width(line), 0.001));
}

/// Draws the chip on the strip: what a permanent's ±1/±1 counters add, net
/// (CR 704.5q). `edge` is its left edge and `mid_y` its middle line, in card
/// widths; `swing` is `cardplate::Corner::chip`, and `aa` is taken by the
/// caller, in uniform control flow.
///
/// A swing is a chip of green or violet stock with the plate's dark body for
/// ink, the way a damaged plate is a hot box with dark digits: at table size
/// the stock is what is read. A swing that nets to neither is a dark chip in
/// the plate's ink, and a `+1/-1` says as much by being neither colour.
fn chip_layer(
    p: vec2<f32>,
    edge: f32,
    mid_y: f32,
    swing: u32,
    aa: f32,
    color: vec3<f32>,
    marks: texture_2d<f32>,
    marks_s: sampler,
) -> vec3<f32> {
    if (swing & SWING_SET) == 0u {
        return color;
    }
    let chip_mid = vec2<f32>(edge + CHIP_W * 0.5, mid_y);
    let chip_half = vec2<f32>(CHIP_W * 0.5, PLATE_H * 0.5);
    let d_chip = sd_round_box(p - chip_mid, chip_half, PLATE_H * 0.28);
    let on_chip = 1.0 - smoothstep(-aa, aa, d_chip);

    let dp = i32(swing & PLATE_SLOT_MASK) - PLATE_BIAS;
    let dt = i32((swing >> PLATE_SLOT_BITS) & PLATE_SLOT_MASK) - PLATE_BIAS;
    var stock = PLATE;
    var figure = INK;
    if dp + dt > 0 {
        stock = GROWN;
        figure = PLATE;
    } else if dp + dt < 0 {
        stock = SHRUNK;
        figure = PLATE;
    }
    var out = mix(color, stock, on_chip * 0.88);
    let chip_rim = 1.0 - smoothstep(-aa, aa, abs(d_chip) - 0.0045);
    out = mix(out, figure, chip_rim * 0.55);

    // A symmetric swing — every `+1/+1` and `-1/-1` counter there is — says
    // its number once: `+2` in a green chip beside a `4/4` reads as what it
    // is, and `+2/+2` in nine pixels would not read at all. The rare
    // lopsided one is written out and shrinks to fit.
    var line = text_signed(vec2<u32>(0u, 0u), dp, true);
    if dp != dt {
        line = text_push(line, GLYPH_SLASH);
        line = text_signed(line, dt, true);
    }
    let room = CHIP_W - 2.0 * PLATE_PAD;
    let hit = text_cover(p, chip_mid, fit(PLATE_CAP, line, room), line, marks, marks_s, aa);
    return mix(out, figure, hit.x * on_chip);
}

/// The plate's width and corner radius for its kind: a chapter is a page,
/// square and barely rounded; everything else is the wide plate.
fn plate_shape(kind: u32) -> vec2<f32> {
    if kind == PLATE_LORE {
        return vec2<f32>(PLATE_H, PLATE_H * 0.10);
    }
    return vec2<f32>(PLATE_W, PLATE_H * 0.28);
}

/// The plate as an object of its own (the owner, 25.09): its body flush
/// right in its quad, over its own drop shadow, as one colour and one
/// coverage for the blend. `p` is the point in the quad, card widths from
/// its top-left corner, and `quad` its size: `cardplate::plate_quad`, whose
/// margins are the count badge's drop and blur, so the two stand at one
/// height over the table.
fn plate_object(
    p: vec2<f32>,
    quad: vec2<f32>,
    word: u32,
    ink: u32,
    aa: f32,
    marks: texture_2d<f32>,
    marks_s: sampler,
) -> vec4<f32> {
    let kind = word >> PLATE_KIND_SHIFT;
    if kind == PLATE_NONE {
        return vec4<f32>(0.0);
    }
    let right = quad.x - BADGE_DROP_X - BADGE_BLUR;
    let top = BADGE_BLUR - BADGE_DROP_Y;
    let shape = plate_shape(kind);
    let half = vec2<f32>(shape.x * 0.5, PLATE_H * 0.5);
    let mid = vec2<f32>(right - half.x, top + half.y);
    // Turned, the face is read a half turn round the body's middle, which
    // the body maps onto itself: the numbers and the damage band turn, and
    // what the plate covers and where its shadow falls do not.
    var q = p;
    if (ink & PLATE_TURNED) != 0u {
        q = 2.0 * mid - p;
    }
    let face = plate_layer(q, right, top, word, ink, aa, marks, marks_s);
    let drop = vec2<f32>(BADGE_DROP_X, BADGE_DROP_Y);
    let d = sd_round_box(p - mid - drop, half, shape.y);
    let fall = 1.0 - smoothstep(0.0, BADGE_BLUR, max(d, 0.0));
    return strip_over_shadow(face.rgb, face.a, SHADOW_DEPTH * fall * fall);
}

/// Draws the plate, its body's right end at `right` and its top at `top`,
/// in card widths: its colour, and how much of the point it covers.
///
/// The two words are `cardplate::Corner::plate_words`: the plate, and its
/// ink, which is the tone with `PLATE_NIGHT` on a sleeping creature, whose
/// numbers are moon-grey. `aa` is taken by the caller, in uniform control
/// flow.
fn plate_layer(
    p: vec2<f32>,
    right: f32,
    top: f32,
    word: u32,
    ink_word: u32,
    aa: f32,
    marks: texture_2d<f32>,
    marks_s: sampler,
) -> vec4<f32> {
    let kind = word >> PLATE_KIND_SHIFT;
    if kind == PLATE_NONE {
        return vec4<f32>(0.0);
    }

    // A chapter is a page: square, barely rounded, and light. Everything else
    // is the wide dark plate. Same corner and same right edge, so the two can
    // never be mistaken for each other and never move.
    let shape = plate_shape(kind);
    let pw = shape.x;
    let radius = shape.y;
    var body = PLATE;
    var accent = INK;
    if kind == PLATE_LORE {
        body = PARCHMENT;
        accent = SEPIA;
    } else if kind == PLATE_LOYALTY {
        // Gilt, and explicitly not a shield — see GILT.
        accent = GILT;
    } else if (ink_word & PLATE_NIGHT) != 0u {
        accent = MOON_INK;
    }

    // Where the table put it: `cardplate::plate_rect`.
    let y0 = top;
    let y1 = y0 + PLATE_H;
    let x1 = right;
    let x0 = x1 - pw;
    let mid = vec2<f32>((x0 + x1) * 0.5, (y0 + y1) * 0.5);
    let half = vec2<f32>(pw * 0.5, PLATE_H * 0.5);

    // ---- the plate
    let d_plate = sd_round_box(p - mid, half, radius);
    let inside = 1.0 - smoothstep(-aa, aa, d_plate);
    if inside <= 0.0 {
        return vec4<f32>(0.0);
    }
    var out = body;

    let a = i32(word & PLATE_SLOT_MASK) - PLATE_BIAS;
    let b = i32((word >> PLATE_SLOT_BITS) & PLATE_SLOT_MASK) - PLATE_BIAS;
    let c = i32((word >> (PLATE_SLOT_BITS * 2u)) & PLATE_SLOT_MASK) - PLATE_BIAS;

    // Damage is a band rising from the bottom of the plate to
    // `damage / toughness`, so what a player reads is how close to lethal
    // this creature is rather than an arithmetic problem in two numerals.
    //
    // The mask is kept, because the numerals below are drawn *through* it:
    // the band is the figure on this plate and a digit standing in it is its
    // ground. That inversion is the whole of the cue — a hot box with dark
    // numbers in it, against a dark box with white ones.
    var band = 0.0;
    if kind == PLATE_FIGHT && c > 0 && b > 0 {
        let share = clamp(f32(c) / f32(b), 0.0, 1.0);
        let lit = max(PLATE_H * share, DAMAGE_FLOOR);
        let level = y1 - lit;
        band = smoothstep(level - aa, level + aa, p.y);
        out = mix(out, HEAT, band);

        // Drawn large — the hover preview, or a camera a player has pushed
        // in — the band is ruled into one row per point of toughness, and
        // the damage becomes a number to count rather than a proportion to
        // judge. The first and last rules are the plate's own edges and are
        // left to it.
        if aa <= TICK_AA && b <= TICK_MAX {
            let row = PLATE_H / f32(b);
            let into = (y1 - p.y) / row;
            let which = round(into);
            let inner = step(0.5, which) * step(which, f32(b) - 0.5);
            let hair = aa * TICK_HAIR;
            let rule = 1.0 - smoothstep(0.0, hair, abs(into - which) * row);
            out = mix(out, body, band * rule * inner * 0.75);
        }
    }

    // The rim, in whichever ink this plate writes with.
    let rim = 1.0 - smoothstep(-aa, aa, abs(d_plate) - 0.0045);
    out = mix(out, accent, rim * 0.55);

    // ---- the numbers
    //
    // Composed first, measured second, drawn third. The figures fill the
    // plate's height unless the line is too wide for it, in which case they
    // shrink until it fits: a `10/10` is smaller than a `2/2` and both are
    // whole, which is the degradation that stays honest — a plate that cut a
    // digit off would be showing a number that is wrong.
    let av = u32(abs(a));
    let roman = kind == PLATE_LORE && av >= 1u && av <= ROMAN_MAX;
    var line = vec2<u32>(0u, 0u);
    var left = 0u;
    if kind == PLATE_FIGHT {
        line = text_signed(line, a, false);
        left = line.y;
        line = text_push(line, GLYPH_SLASH);
        line = text_signed(line, b, false);
    } else if roman {
        for (var k = 0u; k < 3u; k = k + 1u) {
            if k < roman_len(av) {
                line = text_push(line, roman_at(av, k));
            }
        }
        left = line.y;
    } else {
        line = text_number(line, av);
        left = line.y;
    }

    let cap = fit(PLATE_CAP, line, pw - 2.0 * PLATE_PAD);
    let hit = text_cover(p, mid, cap, line, marks, marks_s, aa);
    if hit.x <= 0.0 {
        return vec4<f32>(out, inside);
    }

    // Which ink, and it is two questions. Deathtouch turns the *power* —
    // every character up to the slash — because that is the half of the body
    // the keyword acts through; toxic turns both. And the digits are ground
    // where the damage band is figure, blended rather than switched so that
    // the waterline cuts a numeral cleanly: a `4` half in the band is white
    // above the line and dark below it, which is one more place the level is
    // drawn.
    let tone = ink_word >> TONE_SHIFT;
    var ink = accent;
    if tone == TONE_TOXIC {
        ink = TOXIC;
    } else if tone == TONE_DEADLY && hit.y < f32(left) {
        ink = DEADLY;
    }
    return vec4<f32>(mix(out, mix(ink, body, clamp(band, 0.0, 1.0)), hit.x), inside);
}

/// The fewest permanents a card has to stand for before it says how many,
/// and the most it writes out — `cardplate::COUNT_MIN` and `COUNT_MAX`.
const COUNT_MIN: u32 = 2u;
const COUNT_MAX: u32 = 999u;

/// The count's geometry, in card widths (#261): its box's height, its
/// corner, where its shadow falls and how soft it is, and the widest it
/// grows. `cardplate::BADGE_H`, `BADGE_CORNER`, `BADGE_DROP`, `BADGE_BLUR`
/// and `BADGE_W`. Where it stands on the card is the material's
/// (`cardplate::badge_rect`), so the Rust side moves it.
const BADGE_H: f32 = 0.12;
const BADGE_CORNER: f32 = 0.034;
const BADGE_DROP_X: f32 = 0.006;
const BADGE_DROP_Y: f32 = 0.012;
const BADGE_BLUR: f32 = 0.02;
const BADGE_W: f32 = 0.2006886;

/// How many permanents a merged card stands for, `×54`, at the card's
/// top-right corner (#261, #298; the owner, 25.09): the figures over their
/// own drop shadow, as one colour and one coverage for the blend, and no
/// plate behind them — the owner's placement, outside the card, over its
/// top edge or off its right one. `p` is the point in card widths from the
/// card's top-left corner, `y` down the card. `right` and `top` are where
/// the count's box's right end and top stand.
///
/// The plate's ink and figure height, because it is one of this client's
/// numbers and not something printed. It grows leftwards from `right` with
/// its digits, up to `×99`, and beside the card `right` moves with them so
/// the box starts on the printed border (`cardplate::badge_rect`); three
/// digits are set smaller to fit (`cardplate::badge_cap`). `count` is a uniform, so the
/// early return keeps what follows in uniform control flow. `turned` is
/// `cardplate::PLATE_TURNED`'s: set, the figures are read a half turn round
/// their box's middle, for one looking at the card from its far side.
fn count_badge(
    p: vec2<f32>,
    count: u32,
    turned: u32,
    right: f32,
    top: f32,
    aa: f32,
    marks: texture_2d<f32>,
    marks_s: sampler,
) -> vec4<f32> {
    if count < COUNT_MIN {
        return vec4<f32>(0.0);
    }
    var line = text_push(vec2<u32>(0u, 0u), GLYPH_TIMES);
    line = text_number(line, min(count, COUNT_MAX));
    let wants = text_width(line) * PLATE_CAP / TEXT_CAP;
    let cap = PLATE_CAP * min((BADGE_W - 2.0 * PLATE_PAD) / wants, 1.0);
    let w = max(min(wants + 2.0 * PLATE_PAD, BADGE_W), BADGE_H);
    let mid = vec2<f32>(right - 0.5 * w, top + 0.5 * BADGE_H);

    // The figures' own shadow: the same figures dropped down and a little
    // right, away from the card's print, and softened as far as their
    // distance field reaches, so they read on the felt and on a light print
    // alike.
    let unit = cap / TEXT_CAP;
    let drop = vec2<f32>(BADGE_DROP_X, BADGE_DROP_Y);
    // Turned, the figures turn round the middle of their box, which maps it
    // onto itself; their shadow still falls down the card and right of them.
    var at = p;
    var under = p - drop;
    if turned != 0u {
        at = 2.0 * mid - at;
        under = 2.0 * mid - under;
    }
    let shadow = text_cover(under, mid, cap, line, marks, marks_s, max(aa, 0.08 * unit));
    let shade = SHADOW_DEPTH * 1.4 * shadow.x;
    let hit = text_cover(at, mid, cap, line, marks, marks_s, aa);
    return strip_over_shadow(INK, hit.x, min(shade, 0.9));
}
