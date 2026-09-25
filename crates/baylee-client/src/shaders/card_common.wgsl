// What the table's card shader and its UI twin have to agree about: the shape
// of the printed card, the frame its print sits in, and everything this client
// draws on that frame.
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
// # Why a strip of marks and not more paint
//
// The paper is a *material* — indestructible is what the card is made of,
// hexproof and shroud are what lies over it — and a material composes with at
// most one other material before it stops saying either thing. The keywords
// on the strip are not like that. There are twelve of them — eleven combat
// words and prowess, which earns its slot by being the one a player most
// wants to watch fire — they are equal, a creature can carry six at once, and
// what a player needs is to *count* them and name them. Paint cannot count.
// Marks can: one slot each, always in the same order, so the row is read the
// way a row of icons is read and not the way a colour is guessed at.
//
// The three paper keywords are deliberately absent from the strip. The paper
// already says them, and a mark that repeated it would be the same claim
// twice in two languages.
//
// The strip is drawn here (`marks_strip`) but not by the card: it is an
// object of its own lying on the card, with its own quad and material
// (`marks.wgsl`, `marks_ui.wgsl`), which import this file for the marks.

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

// ---- the frame: a print in a frame of our own
//
// Nothing this client *paints* on the print (#274, `docs/legal.md` §3).
// Scryfall's terms ask that a card image is not covered, cropped, tinted or
// stamped, and the artist's name and the copyright line run along the
// print's bottom edge, which is exactly where a rail and a plate used to be.
// So the quad is still the whole card, 1 × 1/`CARD_ASPECT` card widths, and
// the print is scaled into a window inside it: everything the rules and this
// client say about the card is drawn on the paper around that window. The one
// thing drawn on the print is its own finish (`print_finish`), because a foil
// is what that printing *is*; and light that passes over the whole card and
// leaves nothing behind — the lamp, the arrival sweep, a door.
//
// One object lies over the print, by the owner's decision, and it is not
// this shader's: the keyword strip (`marks_strip`), standing on the art's
// bottom edge. It never lies over the name, the cost, the type line or the
// artist.
//
// The quad does not grow, so no lane, pile, hit test or shadow moves. What
// the frame costs is the print's size: 87.8% of the card's width.
// `cardframe` in client-core is the Rust half of every number here.

/// How wide the frame is beside the print, left and right, in card widths.
///
/// Wider than the black border printed on a modern card (about 0.045), so
/// the frame reads as the card's own paper and not as a second printed
/// border.
const FRAME_SIDE: f32 = 0.061;
/// How deep the frame is above the print.
const FRAME_TOP: f32 = 0.045;
/// The print's width as a share of the card's: what the two sides leave.
///
/// The print keeps 63:88, so its height follows, and what is left of the
/// card's height below it is the ledge (`cardframe::FRAME_FOOT`).
const PRINT_SCALE: f32 = 0.878;

/// Where `uv` falls on the print: 0..1 on both axes inside the window, and
/// running on outside it, so a derivative taken of it is the same everywhere.
fn print_uv(uv: vec2<f32>) -> vec2<f32> {
    let p = vec2<f32>(uv.x, uv.y / CARD_ASPECT);
    return vec2<f32>((p.x - FRAME_SIDE) / PRINT_SCALE, (p.y - FRAME_TOP) * CARD_ASPECT / PRINT_SCALE);
}

/// Signed distance to the print's window, in card widths: negative on the
/// print, positive on the frame. The window is the print's own rounded
/// rectangle, so the scan's corners fall on the frame and never show.
fn window_sdf(uv: vec2<f32>) -> f32 {
    return corner_sdf(print_uv(uv)) * PRINT_SCALE;
}

/// How much of this fragment is print: 1 inside the window, 0 on the frame,
/// antialiased across the window's edge.
fn print_cover(uv: vec2<f32>) -> f32 {
    let aa = max(fwidth(uv.x), 0.0015);
    return 1.0 - smoothstep(-aa, aa, window_sdf(uv));
}

// ---- the text face (#259)
//
// What stands in the window for a card with no print: a token, a printing
// whose art has not arrived, a card the player asked to read as text. It is
// laid out the way a card is — a border, a name bar, an art box, a type bar
// on the keyword strip's seam, a text box — so a text card's bars line up
// with its printed neighbours' in a lane, and it is light where a card is
// light, so a row of text tokens is not a row of holes. The text is not
// drawn here: the table's `Text2d` lines and the overlay's nodes stand on
// these bars. No power and toughness in here, and no state: the ledge's
// plate is the P/T box, and the frame says the rest, as it does for a print.
// `textface` in client-core is the Rust half of every number here.

/// The dark border inside the window, in card widths.
const TEXT_BORDER: f32 = 0.04;
/// Where the type bar's top stands: the keyword strip's seam
/// (`cardrail::strip_bottom`).
const TEXT_SEAM: f32 = 0.7267412;
/// The step the word carries a bar's depth in (`textface::DEPTH_STEP`).
const DEPTH_STEP: f32 = 1.0 / 512.0;
const TEXT_PINLINE: f32 = 0.006;
const TEXT_BOX_GAP: f32 = 0.012;
/// Where the text box ends; under it the foot is the border's colour.
const TEXT_FOOT: f32 = 1.155;
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

/// The face at `uv`, in linear light: the whole window, border included.
/// `word` is `textface::face_word`.
fn text_face(uv: vec2<f32>, word: u32) -> vec3<f32> {
    let p = vec2<f32>(uv.x, uv.y / CARD_ASPECT);
    let aa = max(fwidth(p.x), 0.0015);
    let x0 = FRAME_SIDE + TEXT_BORDER;
    let x1 = FRAME_SIDE + PRINT_SCALE - TEXT_BORDER;
    let top = FRAME_TOP + TEXT_BORDER;
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

// ---- what the frame says
//
// Three registers, all on paper that is ours:
//
// - **The paper** says what the card *is*: its colour is the card's identity
//   (plain, a token, a copy, a commander), indestructible makes it steel, and
//   hexproof or shroud lie over it as a wash. The night a summoning-sick
//   creature lies under is the paper going dark, because the print is not
//   ours to dim.
// - **The rim**, lit from the card's edge inwards, says what this client
//   offers to do with the card or has just been told to: the travelling
//   invitation, the aim, the armed ring, the tap a payment plan will make.
// - **The ledge** under the print carries the numbers.
//
// The sentence the border used to live by still holds, one place over: a
// fact about the card colours the paper, an offer or a deed is light on it.

/// The `cardmat::glow` bits this file reads. `cardmat` is the other half, and
/// a test compares the two.
const GLOW_INDESTRUCTIBLE: u32 = 1u;
const GLOW_HEXPROOF: u32 = 2u;
const GLOW_SHROUD: u32 = 4u;
const GLOW_ACTIVATABLE: u32 = 8u;
const GLOW_SUMMONING_SICK: u32 = 16u;
const GLOW_ARMED: u32 = 32u;
const GLOW_WILL_TAP: u32 = 64u;
const GLOW_COMMANDER: u32 = 128u;
const GLOW_TOKEN: u32 = 1048576u;
const GLOW_COPY: u32 = 2097152u;
const GLOW_REACHABLE: u32 = 8388608u;

/// The frame's own paper: a warm slate, in linear light.
///
/// Mid-tone on purpose. A near-black frame on this felt could show neither a
/// night nor a wash nor an edge, and a light one would out-shout the print it
/// holds.
const FRAME_PAPER: vec3<f32> = vec3<f32>(0.30, 0.29, 0.27);
/// A commander's paper: oxblood.
///
/// Not gilt, which is this client's word for "yours" and is also the armed
/// ring and, near enough, the amber of an offer: a commander with an ability
/// to activate is the common case, and gilt paper under an amber chase would
/// be one colour. One constant, so the owner's answer is one line.
const FRAME_COMMANDER: vec3<f32> = vec3<f32>(0.44, 0.17, 0.15);
/// How dark the paper goes under a summoning-sick creature: 0.30 → 0.14 in
/// linear, about 45 display levels on the plain paper.
const FRAME_NIGHT: f32 = 0.47;
/// The night's white balance. Multiplicative and close to white, so a token's
/// verdigris is still verdigris at night.
const SLEEP_MOON: vec3<f32> = vec3<f32>(0.94, 0.96, 1.0);

/// How far in from the card's edge an offer or a deed is lit, in card widths.
///
/// The frame's thinnest side, so the light has faded out before the window
/// on every side and never reaches the print.
const OFFER_REACH: f32 = 0.045;

/// What the travelling activatable light averages to over its own circuit.
///
/// The chase is a band `pow(1 - 2·dist, 5)` wide riding on a floor of 0.22,
/// scaled by 0.60: its mean over a circuit is 0.22 + 0.60 / 6 = 0.32. A still
/// card (`motion == 0`) is lit at that mean rather than at a frozen crest, so
/// the offer is as strong held still as it is on average when moving.
const CHASE_STILL: f32 = 0.32;

/// The hexproof wash's density, and the thinnest a wisp of it gets.
const WARD_HEX: f32 = 0.55;
const WARD_THIN: f32 = 0.35;
/// The shroud's, which is strictly the stronger of the two and has to look
/// it: a thin shroud beside a deep hexproof wash would say the opposite of
/// what the rules do.
const WARD_SHROUD: f32 = 0.65;

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
// the card instead of jumping at the edges.
fn perimeter(uv: vec2<f32>) -> f32 {
    let d = vec2<f32>(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));
    if d.x < d.y {
        if uv.x < 0.5 {
            return 0.75 + (1.0 - uv.y) * 0.25;
        }
        return 0.25 + uv.y * 0.25;
    }
    if uv.y < 0.5 {
        return uv.x * 0.25;
    }
    return 0.5 + (1.0 - uv.x) * 0.25;
}

/// The paper a card is made of, from what it is.
///
/// A token or a copy is its own paper, a commander is oxblood, and a card that
/// is two of those — a token copy of a commander — wears the stronger on the
/// sides and the top and the other on the ledge (`ledge` 1). Commander is the
/// strongest, then a copy, then a token; token and copy are exclusive already
/// (`board::provenance_of`), and token wins if both ever arrive, because "no
/// cardboard at all" is the stronger claim.
fn frame_paper(glow: u32, ledge: f32) -> vec3<f32> {
    var made = FRAME_PAPER;
    if (glow & GLOW_TOKEN) != 0u {
        made = PAPER_TOKEN;
    } else if (glow & GLOW_COPY) != 0u {
        made = PAPER_COPY;
    }
    if (glow & GLOW_COMMANDER) != 0u {
        return mix(FRAME_COMMANDER, made, ledge * f32((glow & (GLOW_TOKEN | GLOW_COPY)) != 0u));
    }
    return made;
}

/// The whole frame of one card: paper, rim and ledge, everything this client
/// draws on the card and nothing it draws on the print.
///
/// Both card shaders call it and mix it with the print by `print_cover`, so
/// what it returns inside the window is never seen; it is written for the
/// frame alone. `t` is the card's clock (`globals.time * motion`), `m` the
/// motion itself and `now` the unscaled clock, for the one term that has to
/// keep running on a still card.
fn frame_layer(
    uv: vec2<f32>,
    glow: u32,
    plate: u32,
    chips_a: u32,
    chips_b: u32,
    t: f32,
    m: f32,
    now: f32,
    marks: texture_2d<f32>,
    marks_s: sampler,
) -> vec3<f32> {
    let p = vec2<f32>(uv.x, uv.y / CARD_ASPECT);
    let aa = max(fwidth(p.x), 0.0015);

    // ---- the paper
    let foot = FRAME_TOP + PRINT_SCALE / CARD_ASPECT;
    let ledge = smoothstep(foot - aa, foot + aa, p.y);
    var out = frame_paper(glow, ledge);
    if (glow & GLOW_SUMMONING_SICK) != 0u {
        out = out * FRAME_NIGHT * SLEEP_MOON;
    }

    // Indestructible is what the card is made of, so the paper is steel:
    // brushed along the card's long axis and turning slowly under the light.
    if (glow & GLOW_INDESTRUCTIBLE) != 0u {
        let brush = noise(vec2<f32>(uv.x * 120.0, uv.y * 8.0));
        let spec = pow(smoothstep(0.35, 1.0, brush), 3.0);
        let steel = vec3<f32>(0.36, 0.42, 0.50) + vec3<f32>(0.55) * spec;
        let turn = 0.72 + 0.28 * sin(t * 0.8 + uv.y * 3.0);
        out = mix(out, steel * turn, 0.85);
    }

    // Hexproof and shroud are what lies over the card: a wash across the whole
    // frame, wisps for hexproof and a denser haze for shroud, which swallows
    // it (`cardmat::glow_of` never sets both).
    var film = vec3<f32>(0.0);
    var film_cov = 0.0;
    if (glow & GLOW_HEXPROOF) != 0u {
        let n1 = noise(vec2<f32>(uv.x * 6.0 + uv.y * 3.0, p.y * 18.0 + t * 0.50));
        let n2 = noise(vec2<f32>(uv.x * 10.0 - uv.y * 4.0, p.y * 30.0 + t * 0.35));
        let wisp = WARD_THIN + (1.0 - WARD_THIN) * (0.6 * n1 + 0.4 * n2);
        film = vec3<f32>(0.28, 0.86, 0.48);
        film_cov = wisp * WARD_HEX;
    }
    if (glow & GLOW_SHROUD) != 0u {
        let haze = noise(uv * 14.0 + vec2<f32>(t * 0.30, -t * 0.22));
        film = vec3<f32>(0.55, 0.62, 0.92) * (0.55 + 0.45 * haze);
        film_cov = WARD_SHROUD;
    }
    out = mix(out, film, film_cov);

    // ---- the rim: offers and deeds, lit from the card's edge inwards
    let band = 1.0 - smoothstep(0.0, OFFER_REACH, -corner_sdf(uv));
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

    // ---- the ledge: the numbers from the left, the identity at the end
    //
    // The keywords are not here: they are the strip's, an object of its own
    // lying on the art (#274, `marks_strip`).
    let night = (glow & GLOW_SUMMONING_SICK) != 0u;
    out = plate_layer(uv, plate, chips_a, chips_b, night, out, marks, marks_s);
    out = crest_layer(uv, glow, out, marks, marks_s);
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
const MARK_COUNT: u32 = 12u;

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

/// How many marks a row holds; a seventh opens a row above the first.
/// `cardrail::PER_ROW`.
const PER_ROW: u32 = 6u;

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
const ATLAS_CELLS: u32 = 31u;

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
/// movement: the ink in `marks_strip` breathes every mark on the strip —
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
        // Defender: STILL — the scale only. The ink in `marks_strip` breathes
        // this mark with the rest of the row; see the note on this function
        // and `the_ink_below_a_mark_is_not_told_which_mark_it_is`.
        case 10u: { return 1.0; }
        // Prowess: rises, hangs, and comes down — a bonus with a deadline.
        case 11u: { return 1.0 + 0.15 * pow(0.5 + 0.5 * sin(ph * 0.85), 5.0); }
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
        default: { return INK; }
    }
}

/// The keyword strip: a dark plate carrying a card's marks, and the contact
/// shadow it throws on the art round it, as colour and coverage.
///
/// Its own quad draws it (`marks.wgsl` on the table, `marks_ui.wgsl` in the
/// preview), so what it returns is blended over the card rather than mixed
/// into it: the strip is an object lying on the card, not paint on the print
/// (#274). `q` is the point in that quad and `quad` its size, both in card
/// widths from the quad's top-left corner with `y` growing down. The strip
/// stands on the quad's bottom edge — which the quad's owner puts on the
/// seam between the art and the type line — `SHADOW_MARGIN` in from its
/// left, and grows upwards: a seventh mark opens a row above the first, so
/// the row a creature already wears never moves.
///
/// `bits` is the twelve-bit word `cardrail::mark_bits` makes: this file never
/// sees the engine's keyword numbering, or the client's either. `t` is the
/// strip's clock, `globals.time * motion`, and `aa` the antialiasing width in
/// card widths — both taken by the caller, because the two shaders read
/// `globals` from different bind groups and a derivative must be taken in
/// uniform control flow, which the mark a fragment lands in is not. `marks`
/// is a parameter and not a binding for the same reason: this file has no
/// bindings of its own, so `cardmat::tests` can parse it alone.
fn marks_strip(
    q: vec2<f32>,
    quad: vec2<f32>,
    bits: u32,
    t: f32,
    aa: f32,
    marks: texture_2d<f32>,
    marks_s: sampler,
) -> vec4<f32> {
    // Counted in a loop bound at compile time, and not with `countOneBits`.
    // naga lowers that to GLSL's `bitCount`, which arrived in ES 3.10, and it
    // lowers it *unguarded* — WebGL2 compiles ES 3.00, so the browser would
    // reject this shader, the strip's pipeline would fail to build, and no
    // card would carry its marks. The strip has to walk these twelve bits
    // below in any case.
    var n = 0u;
    for (var i = 0u; i < MARK_COUNT; i = i + 1u) {
        if (bits & (1u << i)) != 0u {
            n = n + 1u;
        }
    }
    if n == 0u {
        return vec4<f32>(0.0);
    }

    // `cardrail::strip_rect`, in the quad's own coordinates.
    let rows = (n + PER_ROW - 1u) / PER_ROW;
    let columns = min(n, PER_ROW);
    let pitch = MARK + MARK_GAP;
    let size = vec2<f32>(
        2.0 * STRIP_PAD + f32(columns) * pitch - MARK_GAP,
        2.0 * STRIP_PAD + f32(rows) * pitch - MARK_GAP,
    );
    let x0 = SHADOW_MARGIN;
    let y1 = quad.y;
    let mid = vec2<f32>(x0 + 0.5 * size.x, y1 - 0.5 * size.y);
    let body = sd_round_box(q - mid, 0.5 * size, STRIP_CORNER);
    let cover = 1.0 - smoothstep(-aa, aa, body);

    // The shadow falls on the art round the strip and is gone by the quad's
    // edge; below the strip there is no quad to fall on.
    let fall = 1.0 - smoothstep(0.0, SHADOW_MARGIN, max(body, 0.0));
    let shade = SHADOW_DEPTH * fall * fall;

    // The plate, with an edge a pixel and a half wide, so the strip keeps an
    // outline on artwork as dark as it is.
    let edge = 1.0 - smoothstep(0.0, 1.5 * aa, -body);
    var out = mix(PLATE, STRIP_RIM, edge);

    // Which mark's cell this point is nearest, counted from the bottom-left:
    // the first row stands on the seam and the second opens above it.
    let across = q.x - (x0 + STRIP_PAD);
    let up = (y1 - STRIP_PAD) - q.y;
    let column = u32(clamp(floor((across + 0.5 * MARK_GAP) / pitch), 0.0, f32(columns - 1u)));
    let level = u32(clamp(floor((up + 0.5 * MARK_GAP) / pitch), 0.0, f32(rows - 1u)));
    let k = level * PER_ROW + column;

    // The k-th mark this card actually carries. Twelve iterations, bounded at
    // compile time, no dynamic indexing: the whole reason the strip is a
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
        return strip_over_shadow(out, cover, shade);
    }

    let left = x0 + STRIP_PAD + f32(column) * pitch;
    let top = y1 - STRIP_PAD - f32(level) * pitch - MARK;
    let cell = vec2<f32>((q.x - left) / MARK, (q.y - top) / MARK) - vec2<f32>(0.5);
    let phase = t * BEAT + f32(k) * 0.22;
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
    // sockets and the tower's crenellations at 17 pixels.
    let e = max(0.5 * aa / MARK, 0.012);
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
// tabs under the printed name, which is on the print, so since #274 the
// answer is the frame's own paper (`frame_paper`): the tab's stock became the
// card's. `baylee_client_core::cardcrest` is the Rust half.

/// Where the identity glyphs sit in the atlas, and which is which.
const CREST_BASE: u32 = 28u;
const CREST_TOKEN: u32 = 0u;
const CREST_COPY: u32 = 1u;
const CREST_COMMANDER: u32 = 2u;

/// Nothing to draw.
const CREST_NONE: u32 = 3u;

/// The paper a token and a copy are made of. `cardcrest::PAPER`.
///
/// Linear, and card stock rather than writing paper: displayed around 175 of
/// 255, dark enough to hold ink and to show a night or a wash. Verdigris is a
/// thing conjured rather than printed; violet is what the swing uses for a
/// permanent that is not what it was. A commander's paper is the frame's own
/// constant, `FRAME_COMMANDER`.
const PAPER_TOKEN: vec3<f32> = vec3<f32>(0.396, 0.440, 0.418);
const PAPER_COPY: vec3<f32> = vec3<f32>(0.429, 0.385, 0.506);

/// A crest's square, where the first one's right edge sits, and the air
/// between two, in card widths. `cardcrest::CREST_W`, `CREST_X1`,
/// `CREST_GAP`.
const CREST_W: f32 = 0.085;
const CREST_X1: f32 = 0.955;
const CREST_GAP: f32 = 0.012;

/// The crests' ink, near-black so it holds on all three papers.
/// `cardcrest::CREST_INK`.
const CREST_INK: vec3<f32> = vec3<f32>(0.008, 0.007, 0.006);

/// The identity glyphs, right-aligned on the ledge: a caption for the
/// preview. The paper already says the same thing at table size, so the
/// glyph is not relied on there — a legibility ladder, not the same claim
/// twice.
///
/// Packed from the right, provenance first (`cardcrest::marks`): a lone
/// commander takes the first crest where a lone token would. `glow` is a
/// uniform, so every branch here is uniform, and the atlas is read with an
/// explicit level in `cell_sdf` anyway.
fn crest_layer(
    uv: vec2<f32>,
    glow: u32,
    color: vec3<f32>,
    marks: texture_2d<f32>,
    marks_s: sampler,
) -> vec3<f32> {
    let p = vec2<f32>(uv.x, uv.y / CARD_ASPECT);
    let aa = max(fwidth(p.x), 0.0015);

    var first = CREST_NONE;
    if (glow & GLOW_TOKEN) != 0u {
        first = CREST_TOKEN;
    } else if (glow & GLOW_COPY) != 0u {
        first = CREST_COPY;
    }
    var second = CREST_NONE;
    if (glow & GLOW_COMMANDER) != 0u {
        if first == CREST_NONE {
            first = CREST_COMMANDER;
        } else {
            second = CREST_COMMANDER;
        }
    }
    if first == CREST_NONE {
        return color;
    }

    let foot = FRAME_TOP + PRINT_SCALE / CARD_ASPECT;
    let mid_y = (foot + 1.0 / CARD_ASPECT) * 0.5;
    var out = color;
    for (var n = 0u; n < 2u; n = n + 1u) {
        let which = select(first, second, n == 1u);
        if which == CREST_NONE {
            continue;
        }
        let x1 = CREST_X1 - f32(n) * (CREST_W + CREST_GAP);
        let cell = (p - vec2<f32>(x1 - CREST_W * 0.5, mid_y)) / CREST_W;
        if abs(cell.x) > 0.5 || abs(cell.y) > 0.5 {
            continue;
        }
        let d = cell_sdf(CREST_BASE + which, cell, MARK_RANGE, marks, marks_s);
        let e = max(aa / CREST_W, 0.02);
        out = mix(out, CREST_INK, 1.0 - smoothstep(-e, e, d));
    }
    return out;
}

// ------------------------------------------------------------------ the plate
//
// A creature's power and toughness with the damage marked on it, or a
// planeswalker's loyalty, on the ledge under the print (#274): the plate at
// the ledge's left end and the chip beside it. The Rust half is
// `baylee_client_core::cardplate`, which is where the numbers are packed and
// where every constant below is mirrored and tested.

/// Where the plate starts, how wide it is, its inner margin and its figure
/// height, in card widths. `cardplate::LEDGE_PAD`, `PLATE_W`, `PLATE_PAD`,
/// `PLATE_CAP`.
const LEDGE_PAD: f32 = 0.030;
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
const TEXT_BASE: u32 = 12u;
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

/// The printed body's word. `cardplate::BASE_SET`.
const BASE_SET: u32 = 0x100000u;

/// How large a card has to be *drawn* before the chip writes the printed
/// body. `cardplate::BASE_AA`.
///
/// On the table a card is about 94 physical pixels wide, which would put a
/// second line in the chip at three and make it a smudge on every pumped
/// creature at once. The damage band's rules appear on the same terms and
/// through the same `aa`, so the ledge already behaves this way and a player
/// has already met it.
const BASE_AA: f32 = 0.004;

/// A sleeping creature's plate ink: moon-grey, the night the paper round it
/// is under carried on to the numbers, and still about 5.8:1 on the plate's
/// body.
const MOON_INK: vec3<f32> = vec3<f32>(0.50, 0.54, 0.64);

/// The printed body's ink: the plate's accent, held well back.
///
/// It is the one thing in this corner that is *not* news — it is what the
/// card always said — so it is written at the weight of a caption. Loud
/// enough to read when the card is drawn large enough for it to appear at
/// all, and never loud enough to be mistaken for the body the creature
/// currently has.
const BASE_FADE: f32 = 0.52;

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

/// Draws the plate and the chip beside it, on the ledge.
///
/// The three words are `cardplate::Corner::packed` in order: the plate, the
/// swing and the printed body. Not gated on whether the card has artwork: a
/// card drawn as a flat tint is a card whose art has not loaded, and its
/// body is the thing a player most needs off it. `night` turns the plate's
/// ink to moon-grey on a sleeping creature.
fn plate_layer(
    uv: vec2<f32>,
    word: u32,
    swing: u32,
    base: u32,
    night: bool,
    color: vec3<f32>,
    marks: texture_2d<f32>,
    marks_s: sampler,
) -> vec3<f32> {
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

    // A chapter is a page: square, barely rounded, and light. Everything else
    // is the wide dark plate. Same corner and same left edge, so the two can
    // never be mistaken for each other and never move.
    var pw = PLATE_W;
    var radius = PLATE_H * 0.28;
    var body = PLATE;
    var accent = INK;
    if kind == PLATE_LORE {
        pw = PLATE_H;
        radius = PLATE_H * 0.10;
        body = PARCHMENT;
        accent = SEPIA;
    } else if kind == PLATE_LOYALTY {
        // Gilt, and explicitly not a shield — see GILT.
        accent = GILT;
    } else if night {
        accent = MOON_INK;
    }

    // Centred on the ledge: `cardplate::plate_rect`.
    let foot = FRAME_TOP + PRINT_SCALE / CARD_ASPECT;
    let y0 = (foot + height - PLATE_H) * 0.5;
    let y1 = y0 + PLATE_H;
    let x0 = LEDGE_PAD;
    let x1 = x0 + pw;
    let mid = vec2<f32>((x0 + x1) * 0.5, (y0 + y1) * 0.5);
    let half = vec2<f32>(pw * 0.5, PLATE_H * 0.5);

    var out = color;

    // ---- the chip, beside the plate
    //
    // What the counters did, and — drawn large — what the printing says the
    // body was. It used to be two lines, the swing standing on the plate and
    // the printed body hanging under it; the ledge has room for neither, so
    // they stand beside it, one row each. A swing is a chip of green or
    // violet stock with the plate's dark body for ink, the way a damaged
    // plate is a hot box with dark digits: at table size the stock is what
    // is read. A swing that nets to neither is a dark chip in the plate's
    // ink, and a `+1/-1` says as much by being neither colour.
    let has_swing = (swing & SWING_SET) != 0u;
    let has_base = (base & BASE_SET) != 0u && aa <= BASE_AA;
    if has_swing || has_base {
        let cx0 = LEDGE_PAD + PLATE_W + CHIP_GAP;
        let chip_mid = vec2<f32>(cx0 + CHIP_W * 0.5, mid.y);
        let chip_half = vec2<f32>(CHIP_W * 0.5, PLATE_H * 0.5);
        let d_chip = sd_round_box(p - chip_mid, chip_half, PLATE_H * 0.28);
        let on_chip = 1.0 - smoothstep(-aa, aa, d_chip);

        let dp = i32(swing & PLATE_SLOT_MASK) - PLATE_BIAS;
        let dt = i32((swing >> PLATE_SLOT_BITS) & PLATE_SLOT_MASK) - PLATE_BIAS;
        var stock = body;
        var figure = accent;
        if has_swing && dp + dt > 0 {
            stock = GROWN;
            figure = body;
        } else if has_swing && dp + dt < 0 {
            stock = SHRUNK;
            figure = body;
        }
        out = mix(out, stock, on_chip * 0.88);
        let chip_rim = 1.0 - smoothstep(-aa, aa, abs(d_chip) - 0.0045);
        out = mix(out, figure, chip_rim * 0.55);

        // One row, or two when both are there: the swing on top, because it
        // is news, and the printed body under it, because it is history.
        let room = CHIP_W - 2.0 * PLATE_PAD;
        let rows = select(1.0, 2.0, has_swing && has_base);
        let row_h = PLATE_CAP / rows;
        let row_cap = select(row_h, row_h * 0.85, has_swing && has_base);
        let top = mid.y - PLATE_CAP * 0.5 + row_h * 0.5;
        if has_swing {
            // A symmetric swing — every `+1/+1` and `-1/-1` counter there is
            // — says its number once: `+2` in a green chip beside a `4/4`
            // reads as what it is, and `+2/+2` in nine pixels would not read
            // at all. The rare lopsided one is written out and shrinks to fit.
            var line = text_signed(vec2<u32>(0u, 0u), dp, true);
            if dp != dt {
                line = text_push(line, GLYPH_SLASH);
                line = text_signed(line, dt, true);
            }
            let at = vec2<f32>(chip_mid.x, top);
            let hit = text_cover(p, at, fit(row_cap, line, room), line, marks, marks_s, aa);
            out = mix(out, figure, hit.x * on_chip);
        }
        if has_base {
            let bp = i32(base & PLATE_SLOT_MASK) - PLATE_BIAS;
            let bt = i32((base >> PLATE_SLOT_BITS) & PLATE_SLOT_MASK) - PLATE_BIAS;
            var line = text_signed(vec2<u32>(0u, 0u), bp, false);
            line = text_push(line, GLYPH_SLASH);
            line = text_signed(line, bt, false);
            let at = vec2<f32>(chip_mid.x, top + row_h * (rows - 1.0));
            let hit = text_cover(p, at, fit(row_cap, line, room), line, marks, marks_s, aa);
            out = mix(out, mix(stock, figure, BASE_FADE), hit.x * on_chip);
        }
    }

    // ---- the plate
    let d_plate = sd_round_box(p - mid, half, radius);
    let inside = 1.0 - smoothstep(-aa, aa, d_plate);
    if inside <= 0.0 {
        return out;
    }
    out = mix(out, body, inside * 0.88);

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
        band = inside * smoothstep(level - aa, level + aa, p.y);
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
        return out;
    }

    // Which ink, and it is two questions. Deathtouch turns the *power* —
    // every character up to the slash — because that is the half of the body
    // the keyword acts through; toxic turns both. And the digits are ground
    // where the damage band is figure, blended rather than switched so that
    // the waterline cuts a numeral cleanly: a `4` half in the band is white
    // above the line and dark below it, which is one more place the level is
    // drawn.
    let tone = swing >> TONE_SHIFT;
    var ink = accent;
    if tone == TONE_TOXIC {
        ink = TOXIC;
    } else if tone == TONE_DEADLY && hit.y < f32(left) {
        ink = DEADLY;
    }
    return mix(out, mix(ink, body, clamp(band, 0.0, 1.0)), hit.x);
}

/// The fewest permanents a card has to stand for before it says how many,
/// and the most it writes out — `cardplate::COUNT_MIN` and `COUNT_MAX`.
const COUNT_MIN: u32 = 2u;
const COUNT_MAX: u32 = 999u;

/// The count badge's geometry, in card widths (#261): its height, where its
/// right end and its top stand on the card, its corner, where its shadow
/// falls and how soft it is, and the widest it grows. `cardplate::BADGE_H`,
/// `BADGE_RIGHT`, `BADGE_TOP`, `BADGE_CORNER`, `BADGE_DROP`, `BADGE_BLUR` and
/// `BADGE_W`.
const BADGE_H: f32 = 0.12;
const BADGE_RIGHT: f32 = 0.045;
const BADGE_TOP: f32 = 0.008;
const BADGE_CORNER: f32 = 0.034;
const BADGE_DROP_X: f32 = -0.006;
const BADGE_DROP_Y: f32 = 0.012;
const BADGE_BLUR: f32 = 0.02;
const BADGE_W: f32 = 0.2006886;

/// How many permanents a merged card stands for, `×54`, on a badge of its own
/// hanging off the card's top-left corner (#261): the body over its drop
/// shadow, as one colour and one coverage for the blend. `p` is the point in
/// card widths from the card's top-left corner, `y` down the card, and lies
/// off the card left of its edge, where the badge overhangs.
///
/// The plate's register — its body, its ink, its figure height — because it
/// is one of this client's numbers and not something printed, and the
/// strip's edge, because it is an object lying on the card as the strip is.
/// It grows leftwards with its digits (`cardplate::badge_rect`), off the card
/// and away from the print, up to `×99`; three digits are set smaller to fit
/// (`cardplate::badge_cap`). `count` is a uniform, so the early return keeps
/// what follows in uniform control flow.
fn count_badge(
    p: vec2<f32>,
    count: u32,
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

    let half = vec2<f32>(0.5 * w, 0.5 * BADGE_H);
    let mid = vec2<f32>(BADGE_RIGHT - half.x, BADGE_TOP + half.y);
    let body = sd_round_box(p - mid, half, BADGE_CORNER);
    let cover = 1.0 - smoothstep(-aa, aa, body);

    // The shadow of a thing standing proud of the card rather than lying
    // flat on it: the body itself, dropped down and a little left, away from
    // the print, and softened by `BADGE_BLUR`.
    let drop = vec2<f32>(BADGE_DROP_X, BADGE_DROP_Y);
    let dropped = sd_round_box(p - mid - drop, half, BADGE_CORNER);
    let fall = 1.0 - smoothstep(0.0, BADGE_BLUR, max(dropped, 0.0));
    let shade = SHADOW_DEPTH * fall * fall;

    // The strip's edge, so the badge keeps an outline on a dark card and on
    // the felt it overhangs.
    let edge = 1.0 - smoothstep(0.0, 1.5 * aa, -body);
    var out = mix(PLATE, STRIP_RIM, edge);
    let hit = text_cover(p, mid, cap, line, marks, marks_s, aa);
    out = mix(out, INK, hit.x);
    return strip_over_shadow(out, cover, shade);
}
