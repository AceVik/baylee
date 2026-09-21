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
// keywords on the rail are not like that. There are twelve of them —
// eleven combat words and prowess, which earns its slot by being the one
// a player most wants to watch fire — they are equal, a creature can carry
// six at once, and what a player needs is to
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
/// The metal itself is untouched: `METAL_FLOOR` is added on every frame in
/// both shaders, which is what makes card stock read as coated rather than
/// as paper. This is only the light travelling across it.
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

/// How many keywords can ride the rail.
const MARK_COUNT: u32 = 12u;

/// Where the marks begin in the glow word, and how wide the field is.
///
/// Both card shaders do the shifting at the call site, so nothing below this
/// line knows that the flags for indestructible, hexproof, shroud, a sleeping
/// creature and the three lights this client offers live underneath. The
/// Rust half is
/// `cardmat::glow::MARK_SHIFT`, and a test reads this file to check that the
/// two still agree.
const MARK_SHIFT: u32 = 8u;
const MARK_FIELD: u32 = 0xfffu;

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
// card's UV does, and a mark still reaches no further than its own slot:
// `mark_layer` guillotines at the slot, and the sampler clamps inside the
// cell so a mark can never fetch its neighbour's texels.
//
// # What a motion has to be, at ten pixels
//
// A card on the felt is 86 physical pixels wide and a slot is `RAIL_SLOT` of
// that, so **one cell is ten pixels**, and one cell unit is those ten pixels.
// That number is the whole of this design, and it was arrived at by
// photographing a table rather than by reading this file.
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
// A row of twelve symbols that all move is not a row of symbols. Every
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
// and a full rail of eleven has, on average, nine tenths of one mark moving.

/// One mark's square in the atlas, in texels. `markatlas::CELL`.
const MARK_CELL: f32 = 96.0;

/// How far either side of the outline a *mark* cell encodes, in cell units.
/// `markatlas::RANGE`. The corner's text cells carry their own, shorter one.
const MARK_RANGE: f32 = 0.25;

/// How many cells the atlas holds altogether. `markatlas::CELLS`.
///
/// One row and one texture: the rail's twelve marks, then the corner's
/// fifteen characters. Only the bake rule and the encoded range differ
/// between the two halves, and both of those are settled before a texel is
/// written — so a second texture would be two more bindings in two
/// materials for nothing.
const ATLAS_CELLS: u32 = 30u;

/// The envelope every impulse on the rail shares: up over `a`, held until
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
/// place on the rail where a period is *meant* to line up with the ink
/// pulse, because first strike's claim is about arriving before something,
/// and earliness is only legible against a reference. Do not expect a player
/// to feel it; it costs nothing and is right when lifelink is on the same
/// rail to be early *against*.
fn strike_clock(ph: f32) -> f32 {
    return fract(ph * 0.03979 - 0.0375);
}

/// How big a mark is drawn, as a multiple of its resting size.
///
/// One verb, twelve readings of it. The amplitudes are the same everywhere
/// but the strikes, because a rail whose marks swelled by different amounts
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
/// movement: the ink in `mark_layer` breathes every mark on the row —
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
        // First strike: a blow. Nothing else on the rail arrives this fast.
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
        // Defender: STILL — the scale only. The ink in `mark_layer` breathes
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
/// the rail's marks, the corner's alphabet and the identity column's three
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
/// whole point of the rail — and an implicit derivative asked for there is
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

/// The rail's own reading of a cell: the same fetch, with the pulse on it.
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

/// Draws the rail over `color` and returns what is left.
///
/// `bits` is the twelve-bit mark field, already shifted down out of the glow
/// word: this file never sees the engine's keyword numbering, or the client's
/// either. `t` is `globals.time`, which the two shaders read from two
/// different bind groups — the reason it is a parameter and not a binding,
/// and the same reason `marks` is one: the atlas is bound at a different
/// index in a material bind group than in a UI one, and this file has no
/// bindings of its own so that `cardmat::tests` can parse it alone.
fn mark_layer(
    uv: vec2<f32>,
    bits: u32,
    t: f32,
    color: vec3<f32>,
    marks: texture_2d<f32>,
    marks_s: sampler,
) -> vec3<f32> {
    // Counted in a loop bound at compile time, and not with `countOneBits`.
    // naga lowers that to GLSL's `bitCount`, which arrived in ES 3.10, and it
    // lowers it *unguarded* — WebGL2 compiles ES 3.00, so the browser would
    // reject this shader, the card pipeline would fail to build, and the
    // table would draw no cards at all. The rail has to walk these twelve
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
    //
    // 0.85 and not 0.62, which is the number it carried while the marks were
    // thin strokes. The mix happens in *linear* light and the framebuffer is
    // sRGB, so 0.62 darkens white paper to 168 of 255 — a 27% darkening, not
    // a 62% one — and an ivory wing at 227 stood on it at 1.35:1. At 0.85 the
    // paper reads 118 and the ink about 2:1. Not higher: 0.90 is 98, which is
    // the bar 0.62 was probably meant to be and is a black stripe over dark
    // artwork.
    let mid = vec2<f32>((x0 + x1) * 0.5, (y0 + y1) * 0.5);
    let half = vec2<f32>((x1 - x0) * 0.5 + 0.014, slot * 0.5 + 0.014);
    let plate = sd_round_box(p - mid, half, slot * 0.30);
    var out = mix(color, PLATE, (1.0 - smoothstep(-aa, aa, plate)) * 0.85);

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
    let d = mark_sdf(which, cell, phase, marks, marks_s);
    let accent = mark_color(which);
    // 0.70 and not 0.55, because the halo no longer carries the colour. At
    // table scale a rim two texels wide is sub-pixel and the mark degrades to
    // a coloured pip — which is the design's own honest failure mode, and it
    // now has to happen in the ink or not at all.
    //
    // The pulse is 0.05 and was 0.10: it is the one *continuous* movement on a
    // row whose whole argument is that rest is the state and motion is an
    // event, and a solid silhouette breathes over five times the pixels a
    // stroke did. Not removed, because `strike_clock` is timed to land early
    // against this beat and needs a beat to be early against.
    //
    // It is also not *guarded*, and that is the whole of #102: `which` is
    // read twice above this line and never again, so the twelve marks are
    // drawn by one arithmetic and defender's `mark_pulse` arm — the only
    // STILL one — stills its size and nothing else. `phase` offsets by `k`,
    // the slot, so the row is a staggered breath rather than one; #24's lane
    // wave rides this same term and inherits the same answer, which is why
    // that ticket waited on this one. `cardmat`'s
    // `the_ink_below_a_mark_is_not_told_which_mark_it_is` pins the count, so
    // a guard added here cannot land without saying it is answering #23.
    let ink = mix(INK, accent, 0.70) * (0.95 + 0.05 * sin(phase));

    // Cell units, so the edge is as soft on a card in the preview as on one
    // across the table.
    //
    // Half of `aa`, and both numbers were tuned for a one-pixel stroke that
    // should never quite reach full ink. `aa / slot` is 0.101 cell at the
    // table, so the ramp spanned two whole pixels — on a solid glyph that is
    // not softness, it is a blur, and it is what closed the skull's eye
    // sockets and the tower's crenellations at 17 pixels.
    let e = max(0.5 * aa / slot, 0.012);
    // A rim, not a bloom. At 9 the halo was still 18% of the accent a tenth of
    // a cell out and 5% at the cell wall, so it filled every hole in a solid
    // silhouette — measured on deathtouch, whose eye sockets came back
    // *brighter* than its own ink — and painted each cell its own colour,
    // which put a visible seam between neighbouring marks.
    let halo = exp(-max(d, 0.0) * 24.0) * 0.30;
    out = out + accent * halo;
    out = mix(out, ink, 1.0 - smoothstep(-e, e, d));
    return out;
}

// --------------------------------------------------------- the identity slips
//
// Two questions about what a permanent *is*, drawn as paper tabs along the
// card's left margin under the printed name: is it a commander (CR 903.3),
// and is the card it looks like its own (CR 111.1, CR 707.2)?
//
// This is their third home and the second one the owner sent back. They were
// on the **top edge** — a procedural crown centred on it, a disc or a pair of
// cards in the corner — and that edge is the title bar, so the marks sat on
// the printed name. They moved to a **column in the right margin**, on the
// plate's centre line, so the whole right corner read as one column; the
// answer to that was *„Die Position gefällt mir noch nicht"*, and it is a
// fair one, because that corner had collected the numbers and what counters
// did to them, and none of this is a number.
//
// So: slips, in the one band of a card that carries neither the name nor the
// numbers. `baylee_client_core::cardcrest` is the Rust half and the mirror
// test in `cardmat.rs` is what keeps the two saying the same thing.
//
// # Three things the move changed, each of which withdrew an argument
//
// **They pack.** The column reserved a row per question so that position
// alone told a shield from a squirrel at the seven physical pixels a slot
// gets on a table card; a lone commander now takes the first slip, where a
// lone token would. What took position's job is `slip_paper` — a colour per
// kind — and the colour is the whole reason the packing is safe.
//
// **They are coloured at all**, which the column refused on the grounds that
// every hue in this client is spoken for and a fourth reading of the palette
// is a claim nobody can look up. What makes it affordable is that the colour
// is the *stock*, not ink added to the mark: three papers in one place are
// an alphabet of three.
//
// **And they move.** The column deliberately did not — a permanent stops
// being a commander only by ceasing to be that permanent (CR 400.7), so a
// mark that breathed would be promising a change that cannot come. The owner
// asked for an animation. What ships is the rarest motion on the card, a
// half-minute apart against the rail's slowest keyword at 24.1 s, and it is
// light crossing paper rather than the mark itself changing.
//
// # Why the paper is card stock and not writing paper
//
// These constants are **linear** and the framebuffer converts, which is the
// same trap the rail's plate comment records from the other side. The first
// draft's papers were 0.72–0.92, which display at 221 to 246 of 255, and a
// sheen mixed 45% toward white moves a sheet that pale by **16 levels** —
// under the 20 a mark that does not move at all already swings from the ink
// pulse. At 55% of those values the paper displays around 175 and the same
// sheen moves 44 to 70. A slip too pale cannot catch the light, and no
// amount of sheen fixes it.

/// A mark's square inside its slip, and the paper around it, in card widths.
///
/// `SLIP_PAD_X` is the wider of the two on purpose: a tab is longer than it
/// is deep, and at this size the shape of the paper reads long before what is
/// printed on it.
const SLIP_SLOT: f32 = 0.092;
const SLIP_PAD_X: f32 = 0.034;
const SLIP_PAD_Y: f32 = 0.016;
const SLIP_GAP: f32 = 0.010;

/// Where the first slip's left edge and every slip's top edge sit, in card
/// widths from the card's own corner.
///
/// `SLIP_INSET` is the rail's, because the rail runs from the same margin
/// along the bottom edge. `SLIP_TOP` places the slip's **top** edge just
/// below a modern frame's title bar; the tab hangs down from there into the
/// top corner of the art, which is what a tab clipped to a document does and
/// what the rail and the plate already do at the other end. It is a
/// constant: an old border and a full art card put something different
/// there, and a slip that chased the frame would move when a player swapped
/// one printing for another.
const SLIP_INSET: f32 = 0.052;
const SLIP_TOP: f32 = 0.151;

/// How often a slip catches the light, over the rail's own beat, and how far
/// the band mixes its paper toward white.
///
/// `fract(ph * K)` has period `1 / (BEAT * K)` seconds — 0.8696 / K, and not
/// the 5.464 / K it would be if the term were a `sin`. At this rate that is
/// thirty seconds.
const SLIP_SHEEN_RATE: f32 = 0.0290;
const SLIP_SHEEN: f32 = 0.45;

/// The ink every mark is printed in, on all three papers.
///
/// Darker than `SEPIA`, this file's other ink on paper, because these papers
/// are darker than the saga's parchment: `SEPIA` on this stock measures
/// 2.0:1 and this pair is 5.6:1 at worst. `cardcrest::SLIP_INK`.
const SLIP_INK: vec3<f32> = vec3<f32>(0.035, 0.030, 0.025);

/// How many slips a card can wear. `cardcrest::MAX_SLIPS`.
///
/// Two, because there are two questions — is it a commander, and is the card
/// it looks like its own — and provenance is one value of three rather than
/// three flags. It is a constant and not a count of what is set, so the loop
/// below is bounded at compile time whatever the packing says.
const SLIP_MAX: u32 = 2u;

/// Where the slips' glyphs sit in the atlas, and which is which.
const CREST_BASE: u32 = 27u;
const CREST_TOKEN: u32 = 0u;
const CREST_COPY: u32 = 1u;
const CREST_COMMANDER: u32 = 2u;

/// Nothing to draw in this slip.
const CREST_NONE: u32 = 3u;

/// The stock each mark is printed on. `cardcrest::SLIP_PAPER`.
///
/// Gilt is already this client's word for *this one of yours* and rims the
/// viewing seat's own mat; verdigris is a thing conjured rather than printed;
/// violet is what the swing uses for a permanent that is not what it was. The
/// three stand at least 106 degrees of hue apart, which is what the packing
/// spends and what a test on the Rust side holds them to.
///
/// Three constants and a switch rather than three literals in the switch,
/// because a `vec3` inside a function body is not a thing the mirror test
/// can read — and a colour nobody mirrors is a colour that drifts.
const SLIP_PAPER_TOKEN: vec3<f32> = vec3<f32>(0.396, 0.440, 0.418);
const SLIP_PAPER_COPY: vec3<f32> = vec3<f32>(0.429, 0.385, 0.506);
const SLIP_PAPER_COMMANDER: vec3<f32> = vec3<f32>(0.495, 0.418, 0.220);

fn slip_paper(which: u32) -> vec3<f32> {
    switch which {
        case 0u: { return SLIP_PAPER_TOKEN; }
        case 1u: { return SLIP_PAPER_COPY; }
        default: { return SLIP_PAPER_COMMANDER; }
    }
}

/// The identity slips: at most two paper tabs under the card's printed name.
///
/// Takes the three bools rather than the glow word, the way the rail takes
/// its bits pre-shifted: which bit of that word means what is the Rust half's
/// business (`cardmat::glow::COMMANDER`, `::TOKEN` and `::COPY`). Token and
/// copy are exclusive already — `board::provenance_of` returns one value of
/// three — and `is_token` wins here regardless, because "no cardboard at all"
/// is the stronger claim and a slip drawing both would be saying neither.
fn identity_layer(
    uv: vec2<f32>,
    is_commander: bool,
    is_token: bool,
    is_copy: bool,
    t: f32,
    color: vec3<f32>,
    marks: texture_2d<f32>,
    marks_s: sampler,
) -> vec3<f32> {
    // Packed left to right, provenance first: it is the one a table actually
    // wears, so the mark that is usually there is the one anchored to the
    // margin it hangs from. `cardcrest::marks` is this, in Rust.
    var first = CREST_NONE;
    if is_token {
        first = CREST_TOKEN;
    } else if is_copy {
        first = CREST_COPY;
    }
    var second = CREST_NONE;
    if is_commander {
        if first == CREST_NONE {
            first = CREST_COMMANDER;
        } else {
            second = CREST_COMMANDER;
        }
    }
    if first == CREST_NONE {
        return color;
    }

    // Width-units, so a slot is square — the rail's change of variables.
    let p = vec2<f32>(uv.x, uv.y / CARD_ASPECT);

    // Uniform control flow, before any branch on where the fragment landed:
    // a derivative asked for inside one is undefined on half the backends
    // this ships to.
    let aa = max(fwidth(p.x), 0.0015);
    let ph = t * BEAT;

    let w = SLIP_SLOT + 2.0 * SLIP_PAD_X;
    let h = SLIP_SLOT + 2.0 * SLIP_PAD_Y;

    var out = color;

    for (var n = 0u; n < SLIP_MAX; n = n + 1u) {
        var which = first;
        if n == 1u {
            which = second;
        }
        if which == CREST_NONE {
            continue;
        }

        let x0 = SLIP_INSET + f32(n) * (w + SLIP_GAP);
        let y0 = SLIP_TOP;
        let mid = vec2<f32>(x0 + w * 0.5, y0 + h * 0.5);

        // The paper, opaque where it lands. The rail's plate is a partial mix
        // because it is a disc laid over artwork the player still wants to
        // see; a slip is a piece of paper on top of the card, and paper that
        // the picture shows through is not paper.
        let edge = sd_round_box(p - mid, vec2<f32>(w * 0.5, h * 0.5), SLIP_PAD_Y);
        let on = 1.0 - smoothstep(-aa, aa, edge);
        if on <= 0.0 {
            continue;
        }

        // ---- the sheen
        //
        // A band of light crossing the tab once, on the envelope every
        // impulse on the rail shares: up over a fiftieth of the period, held
        // to a twentieth, gone by a thirtieth past that — about two and a
        // half seconds of the thirty, so a slip is still for nine tenths of
        // its life. `k` is what carries the band across; the envelope only
        // fades it in and out, and driving the position from the envelope
        // instead would send the band back the way it came.
        //
        // Phase-offset per slip, the rail's own rule: two tabs catching the
        // light together would read as the card flashing.
        let rise = 0.02;
        let hold = 0.05;
        let fall = 0.03;
        let f = fract(ph * SLIP_SHEEN_RATE + f32(n) * 0.37);
        let amp = mark_event(f, rise, hold, fall);

        // `hold + fall` is where the envelope reaches nothing, so `k` runs
        // 0 to 1 over exactly the life of the event and not over some
        // number that happens to look right beside it. The band starts a
        // quarter of a tab off the left edge and travels one and a half
        // tabs, so it is fully off both ends rather than fading in place.
        let k = clamp(f / (hold + fall), 0.0, 1.0);
        let band = (p.x - x0) / w - (k * 1.5 - 0.25);
        let lit = exp(-band * band * 30.0) * amp;

        var stock = slip_paper(which);
        stock = mix(stock, vec3<f32>(1.0), lit * SLIP_SHEEN);
        out = mix(out, stock, on);

        // ---- the mark
        let mx = x0 + SLIP_PAD_X;
        let my = y0 + SLIP_PAD_Y;
        if p.x < mx || p.x > mx + SLIP_SLOT || p.y < my || p.y > my + SLIP_SLOT {
            continue;
        }

        let cell = vec2<f32>((p.x - mx) / SLIP_SLOT, (p.y - my) / SLIP_SLOT)
            - vec2<f32>(0.5);
        let d = cell_sdf(CREST_BASE + which, cell, MARK_RANGE, marks, marks_s);

        // Cell units, so the edge is as soft in the preview as across the
        // table.
        let e = max(aa / SLIP_SLOT, 0.02);
        out = mix(out, SLIP_INK, 1.0 - smoothstep(-e, e, d));
    }

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
const PLATE_PAD: f32 = 0.020;

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
/// order and `markatlas` bakes them into the same row as the rail's marks.
const TEXT_BASE: u32 = 12u;
const TEXT_COUNT: u32 = 15u;
const GLYPH_MINUS: u32 = 10u;
const GLYPH_SLASH: u32 = 11u;
const GLYPH_PLUS: u32 = 12u;
const GLYPH_I: u32 = 13u;
const GLYPH_V: u32 = 14u;

/// How far either side of an outline a text cell's field reaches, and how
/// tall a lining figure is — both in cell units, both mirrored.
const TEXT_RANGE: f32 = 0.10;
const TEXT_CAP: f32 = 0.57665;

/// The advances, in cell units. Six numbers and not fifteen: the digits are
/// **tabular**, which is the difference between a creature growing from
/// `9/9` to `10/10` and one that also shunts its own slash sideways.
const TEXT_ADV_DIGIT: f32 = 0.45125;
const TEXT_ADV_MINUS: f32 = 0.28880;
const TEXT_ADV_SLASH: f32 = 0.26980;
const TEXT_ADV_PLUS: f32 = 0.45600;
const TEXT_ADV_I: f32 = 0.28025;
const TEXT_ADV_V: f32 = 0.55765;

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

/// Where the swing line stands and how tall its figures are, in card widths.
/// `cardplate::SWING_GAP` and `SWING_H`.
const SWING_GAP: f32 = 0.012;
const SWING_H: f32 = 0.04650;

/// The printed body's line, under the plate. `cardplate::BASE_*`.
const BASE_SET: u32 = 0x100000u;
const BASE_GAP: f32 = 0.006;
const BASE_H: f32 = 0.040;

/// How large a card has to be *drawn* before the printed body appears.
///
/// On the table a card is about 150 physical pixels wide, which puts this
/// line at five and makes it a smudge on every pumped creature at once. The
/// damage band's rules appear on the same terms and through the same `aa`,
/// so the corner already behaves this way and a player has already met it.
const BASE_AA: f32 = 0.004;

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
/// mark on a rail that holds twelve — the number *is* the thing the keyword
/// changes the meaning of, so the colour belongs on it.
const DEADLY: vec3<f32> = vec3<f32>(0.42, 0.86, 0.45);

/// Toxic red, on both numbers. Nothing constructs it yet; see
/// `cardplate::Tone::Toxic`.
const TOXIC: vec3<f32> = vec3<f32>(0.95, 0.35, 0.33);

/// A swing that grew the creature, and one that shrank it.
///
/// The two colours the counter chips carried before the numerals took their
/// place — growth green and bruise violet — kept because they were the one
/// part of a chip that was read at a glance, and because a `+2/+2` and a
/// `-2/-2` differ by a character eleven pixels tall otherwise.
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
        default: { return TEXT_ADV_V; }
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
/// alphabet is fifteen characters long and eight of them have to fit in a
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

/// Draws the plate, the swing line above it and the printed body below it.
///
/// The three words are `cardplate::Corner::packed` in order. Not gated on whether the card has artwork: a
/// card drawn as a flat tint is a card whose art has not loaded, and its
/// body is the thing a player most needs off it.
fn plate_layer(
    uv: vec2<f32>,
    word: u32,
    swing: u32,
    base: u32,
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
    // is the wide dark plate. Same corner and same right edge, so the two can
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
    }

    let x1 = 1.0 - PLATE_INSET;
    let x0 = x1 - pw;
    let y1 = height - PLATE_INSET;
    let y0 = y1 - PLATE_H;
    let mid = vec2<f32>((x0 + x1) * 0.5, (y0 + y1) * 0.5);
    let half = vec2<f32>(pw * 0.5, PLATE_H * 0.5);

    var out = color;

    // ---- the swing, above the plate
    //
    // The net that this permanent's ±1/±1 counters added, one size down and
    // on the plate's own centre line, so the corner reads as one column. It
    // is drawn first because it is *outside* the plate and the plate's own
    // early return below is on the plate's rectangle.
    if (swing & SWING_SET) != 0u {
        let dp = i32(swing & PLATE_SLOT_MASK) - PLATE_BIAS;
        let dt = i32((swing >> PLATE_SLOT_BITS) & PLATE_SLOT_MASK) - PLATE_BIAS;
        var line = text_signed(vec2<u32>(0u, 0u), dp, true);
        line = text_push(line, GLYPH_SLASH);
        line = text_signed(line, dt, true);
        let at = vec2<f32>(mid.x, y0 - SWING_GAP - SWING_H * 0.5);
        // Never wider than the plate it explains. `+2/-1` is six characters
        // against the plate's usual three, and unclamped it hung off the
        // card's right edge — an appendage has to stay inside the thing it
        // is an appendage to.
        let hit = text_cover(p, at, fit(SWING_H, line, pw), line, marks, marks_s, aa);
        // Green for a creature that grew, violet for one that shrank, and
        // the plate's ink for the rare swing that nets to neither — a
        // `+1/-1` says as much by being neither colour.
        var tint = accent;
        if dp + dt > 0 {
            tint = GROWN;
        } else if dp + dt < 0 {
            tint = SHRUNK;
        }
        out = mix(out, tint, hit.x);
    }

    // ---- the printed body, below the plate
    //
    // The number the plate is *standing on*: a real card prints its power
    // and toughness in this exact corner, so the plate covers them, and a
    // player looking at a 5/5 cannot see that it was printed a 2/2. Drawn
    // only when the two differ — a creature at its printed size says it
    // once — and only when the card is drawn large enough to read a line
    // this small.
    if (base & BASE_SET) != 0u && aa <= BASE_AA {
        let bp = i32(base & PLATE_SLOT_MASK) - PLATE_BIAS;
        let bt = i32((base >> PLATE_SLOT_BITS) & PLATE_SLOT_MASK) - PLATE_BIAS;
        var line = text_signed(vec2<u32>(0u, 0u), bp, false);
        line = text_push(line, GLYPH_SLASH);
        line = text_signed(line, bt, false);
        let at = vec2<f32>(mid.x, y1 + BASE_GAP + BASE_H * 0.5);
        let hit = text_cover(p, at, fit(BASE_H, line, pw), line, marks, marks_s, aa);
        out = mix(out, accent, hit.x * BASE_FADE);
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

    let cap = fit(PLATE_H - 2.0 * PLATE_PAD, line, pw - 2.0 * PLATE_PAD);
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
