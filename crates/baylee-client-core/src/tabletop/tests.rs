use super::*;
use crate::layout::CARD_WIDTH;

/// sRGB byte to linear light, and back. The GPU blends in linear and the
/// texture is sRGB, so a claim about what a player *sees* has to make the
/// same round trip the hardware does — comparing the two byte values
/// directly is the mistake that let an invisible well ship.
fn to_linear(byte: f32) -> f32 {
    let c = byte / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn to_srgb(linear: f32) -> f32 {
    let c = linear.clamp(0.0, 1.0);
    let v = if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055f32.mul_add(c.powf(1.0 / 2.4), -0.055)
    };
    v * 255.0
}

/// One texel composited over a background, the way the blend does it.
fn over(texel: [f32; 4], ground: [f32; 3]) -> [f32; 3] {
    let mut out = [0.0; 3];
    for channel in 0..3 {
        let src = to_linear(texel[channel] * 255.0);
        let dst = to_linear(ground[channel]);
        out[channel] = to_srgb(texel[3].mul_add(src, (1.0 - texel[3]) * dst));
    }
    out
}

/// The table beside a seat mat, measured off a screenshot of a live
/// table at two points: in shadow, and where the lamp reaches.
const TIMBER_DARK: [f32; 3] = [22.0, 15.0, 11.0];
const TIMBER_LIT: [f32; 3] = [52.0, 37.0, 26.0];

/// The brightest texel of a well, and the one in the middle of its fill.
fn well_lip_and_fill() -> ([f32; 4], [f32; 4]) {
    let well = card_well(128, 179, CARD_CORNER_FRACTION, ZoneMark::Library);
    let lip = pixels(&well)
        .map(|(_, _, rgba)| rgba)
        .max_by(|a, b| a[0].total_cmp(&b[0]))
        .expect("a well has pixels");
    (lip, well.pixel(64, 20))
}

/// A card's corner as a fraction of its short side; the renderer passes
/// the same number, from `CARD_CORNER / CARD_WIDTH`.
const CARD_CORNER_FRACTION: f32 = 0.0476;

#[test]
fn an_empty_place_can_be_seen_on_the_wood_it_is_cut_into() {
    let (lip, fill) = well_lip_and_fill();
    for ground in [TIMBER_DARK, TIMBER_LIT] {
        let drawn = over(lip, ground);
        let apart = (0..3)
            .map(|c| (drawn[c] - ground[c]).abs())
            .fold(0.0f32, f32::max);
        assert!(
            apart > 40.0,
            "the lip is {apart:.0}/255 from {ground:?}, which is a well nobody can find"
        );
    }
    // And the half that could not do it alone. This is not a weaker
    // version of the assertion above: it is the measurement that says
    // *why* there is a lip at all, and it fails if somebody decides the
    // fill can carry the reading by getting darker.
    let alone = over(fill, TIMBER_DARK);
    let apart = (0..3)
        .map(|c| (alone[c] - TIMBER_DARK[c]).abs())
        .fold(0.0f32, f32::max);
    assert!(
        apart < 12.0,
        "a fill {apart:.0}/255 from the table is doing the lip's work"
    );
}

#[test]
fn a_well_stays_quieter_than_the_seat_rim_beside_it() {
    // The gilt of the local seat's rim, measured at the same table.
    const RIM: [f32; 3] = [110.0, 79.0, 42.0];
    let (lip, _) = well_lip_and_fill();
    let drawn = over(lip, TIMBER_DARK);
    let brightness = |c: [f32; 3]| 0.2126f32.mul_add(c[0], 0.7152 * c[1] + 0.0722 * c[2]);
    assert!(
        brightness(drawn) < brightness(RIM),
        "the well draws at {drawn:?}, brighter than the rim at {RIM:?} — \
             a place for a pile must not outshine whose seat it is"
    );
}

/// Every zone's mark is in the middle of its well, inside the lip, and
/// unlike every other zone's.
///
/// The last part is the one worth a test. A mark exists to answer "which
/// zone is this" at a glance from across a table, so four marks that
/// covered nearly the same texels would be four wells again with extra
/// arithmetic — and that is exactly what a small change to one of the
/// shapes could quietly produce.
/// A texel back as the bytes it is stored as, so two of them may be
/// compared for equality without comparing floats.
fn quantised(rgba: [f32; 4]) -> [u8; 4] {
    rgba.map(|v| (v * 255.0 + 0.5) as u8)
}

#[test]
fn each_zone_wears_its_own_mark_and_wears_it_in_the_middle() {
    let marks = [
        ZoneMark::Library,
        ZoneMark::Graveyard,
        ZoneMark::Exile,
        ZoneMark::Command,
    ];
    let wells: Vec<Texture> = marks
        .iter()
        .map(|&mark| card_well(128, 179, CARD_CORNER_FRACTION, mark))
        .collect();
    // The middle box the mark is allowed to reach: `MARK_HALF` of the
    // short side either way, plus a texel for the samples' own edge.
    let inside = |x: u32, y: u32| {
        let half = MARK_HALF * 128.0 + 1.0;
        (x as f32 - 63.5).abs() <= half && (y as f32 - 89.5).abs() <= half
    };
    // The alpha the well carries with nothing drawn on it, read where no
    // lip reaches and no mark may: past the lip, above the box.
    let plain = wells[0].pixel(64, 20)[3];

    for (mark, well) in marks.iter().zip(&wells) {
        let drawn = (0..179 * 128)
            .filter(|i| {
                let (x, y) = (i % 128, i / 128);
                inside(x, y) && well.pixel(x, y)[3] > plain + 0.02
            })
            .count();
        assert!(
            drawn > 400,
            "{mark:?} drew {drawn} texels, which is nothing"
        );
    }

    // Outside that box every well is the same well. A mark that ran into
    // the lip would fail here rather than merely look wrong.
    for (mark, well) in marks.iter().zip(&wells).skip(1) {
        for i in 0..179 * 128 {
            let (x, y) = (i % 128, i / 128);
            if inside(x, y) {
                continue;
            }
            assert_eq!(
                quantised(well.pixel(x, y)),
                quantised(wells[0].pixel(x, y)),
                "{mark:?} reached ({x}, {y}), outside its own box"
            );
        }
    }

    for (i, a) in marks.iter().enumerate() {
        for (j, b) in marks.iter().enumerate().skip(i + 1) {
            let differ = (0..179 * 128)
                .filter(|k| {
                    let (x, y) = (k % 128, k / 128);
                    inside(x, y)
                        && quantised(wells[i].pixel(x, y)) != quantised(wells[j].pixel(x, y))
                })
                .count();
            assert!(
                differ > 300,
                "{a:?} and {b:?} differ on only {differ} texels — they read as one shape"
            );
        }
    }
}

#[test]
fn a_well_is_a_rim_and_not_a_disc() {
    let well = card_well(128, 179, CARD_CORNER_FRACTION, ZoneMark::Library);
    // Across the middle: transparent outside, bright at the lip, and
    // back down to the fill in the centre. A disc would rise once.
    let row: Vec<f32> = (0..128).map(|x| well.pixel(x, 30)[0]).collect();
    let brightest = |slice: &[f32]| {
        slice
            .iter()
            .copied()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .expect("a half-row has pixels")
    };
    // Both halves, because the lip runs all the way round and a test that
    // took the whole row's maximum would only ever find one of them.
    let left = brightest(&row[..64]);
    let right = brightest(&row[64..]);
    assert!(
        left.0 < 32,
        "the left lip peaks at {}, not near its edge",
        left.0
    );
    assert!(
        right.0 + 64 > 95,
        "the right lip peaks at {}, not near its edge",
        right.0 + 64
    );
    assert!(
        row[64] < left.1.min(right.1) * 0.5,
        "the middle is {:.3} against lips of {:.3} and {:.3} — that is a disc",
        row[64],
        left.1,
        right.1
    );
}

/// Every pixel of a texture, as `(x, y, rgba)`.
fn pixels(t: &Texture) -> impl Iterator<Item = (u32, u32, [f32; 4])> + '_ {
    (0..t.height).flat_map(move |y| (0..t.width).map(move |x| (x, y, t.pixel(x, y))))
}

/// Hue in degrees and saturation, the two channels a colour is recognised
/// by across a table. Value is left out deliberately: a rim and a wash at
/// the same hue read as the same colour whichever is brighter.
fn hue_sat(rgb: [f32; 3]) -> (f32, f32) {
    let [r, g, b] = rgb;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let chroma = max - min;
    if chroma < 1e-6 || max < 1e-6 {
        return (0.0, 0.0);
    }
    let hue = 60.0
        * if (max - r).abs() < 1e-6 {
            ((g - b) / chroma).rem_euclid(6.0)
        } else if (max - g).abs() < 1e-6 {
            (b - r) / chroma + 2.0
        } else {
            (r - g) / chroma + 4.0
        };
    (hue.rem_euclid(360.0), chroma / max)
}

/// The shorter way round the wheel.
fn hue_gap(a: f32, b: f32) -> f32 {
    let d = (a - b).abs().rem_euclid(360.0);
    d.min(360.0 - d)
}

/// Every wash, with the name the failure message needs.
const WASHES: [(&str, [f32; 3]); 4] = [
    ("COOL", COOL),
    ("CANDLE", CANDLE),
    ("EMBER", EMBER),
    ("DUSK", DUSK),
];

/// Below this a colour has no hue worth confusing — the pie's parchment
/// white and slate black are both under it, which is why neither appears
/// in the failures this test can produce.
const HAS_A_HUE: f32 = 0.35;

/// Relative luminance, the channel the eye actually judges "how dark" by.
fn luma(rgba: [f32; 4]) -> f32 {
    0.2126f32.mul_add(rgba[0], 0.7152f32.mul_add(rgba[1], 0.0722 * rgba[2]))
}

/// Standard deviation, for the contrast measurements below.
fn deviation(values: &[f32]) -> f32 {
    let n = values.len() as f32;
    let mean = values.iter().sum::<f32>() / n;
    (values.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / n).sqrt()
}

/// The redesign replaces casino green with cool mineral and warm metal.
/// Both hue and saturation remain bounded; this is not permission for
/// a neutral black hole or a saturated blue field behind blue cards.
#[test]
fn the_baize_is_cool_mineral_under_warm_metal() {
    for cloth in [FELT_DEEP, FELT_CLOTH, FELT_WORN] {
        let (hue, sat) = hue_sat(cloth);
        assert!(
            (185.0..205.0).contains(&hue),
            "{cloth:?} is at {hue}°, outside the mineral palette"
        );
        assert!(
            (0.30..0.50).contains(&sat),
            "{cloth:?} is {sat} saturated — cloth, not a swatch"
        );
    }
    // Warm metal against cool cloth. A rail the same hue as the felt is
    // a table with no rail.
    for hide in [RAIL_HIDE, RAIL_LIP, APRON] {
        let (hue, _) = hue_sat(hide);
        assert!(
            (10.0..45.0).contains(&hue),
            "the rail at {hue}° is not warm metal"
        );
    }
    // The wall has to be darker than the top it hangs from, or the slab
    // has no thickness to see. Nothing lights this stage, so this
    // difference *is* the third dimension.
    assert!(
        luma([APRON[0], APRON[1], APRON[2], 1.0])
            < luma([RAIL_HIDE[0], RAIL_HIDE[1], RAIL_HIDE[2], 1.0]),
        "the apron has to be darker than the rail above it"
    );
}

/// And it is *rough*: a weave you can see at the distance a card is read.
///
/// The measurement is at card scale deliberately — a grain that only
/// exists at texel scale is a grain nobody ever sees, and the same
/// mistake was caught once already on the wood this replaces.
#[test]
fn the_baize_has_a_tooth_at_card_scale() {
    let cloth = felt(256);
    // A card is `CARD_WIDTH` of a table about 30 units across, so it is
    // roughly a thirtieth of this texture: sample a card-sized patch.
    let side = (256.0 * CARD_WIDTH / 30.0).round() as u32;
    assert!(side >= 6, "a card is {side} texels — too few to measure");
    let patch: Vec<f32> = (0..side)
        .flat_map(|y| (0..side).map(move |x| (x + 100, y + 100)))
        .map(|(x, y)| luma(cloth.pixel(x, y)))
        .collect();
    let rough = deviation(&patch);
    assert!(
        rough > 0.002,
        "the cloth is flat at card scale ({rough}) — that is paint, not felt"
    );
    assert!(
        rough < 0.030,
        "the cloth is louder than the cards on it ({rough})"
    );
}

/// A table has rounded corners, and neither a chamfer nor a stadium.
///
/// It was a racetrack at 0.42, chosen when the corners were the *only*
/// place the sky could be seen. `AIR` now leaves a band all the way
/// round, so the corner is free to be what a table's corner is — and the
/// owner's word on the racetrack was "less border corner". Both bounds
/// are here because both failures are real: a corner too small is a
/// rectangle with the edges filed off, and one too large eats the play
/// area at the ends of the mats.
#[test]
fn a_table_has_a_corner_and_not_a_chamfer() {
    let span = Vec2::new(34.0, 26.0);
    let r = table_corner(span);
    assert!(
        r > span.min_element() * 0.04,
        "a {r} corner on a {span:?} table reads as a bevel, not a corner"
    );
    assert!(
        r < span.min_element() * 0.10,
        "a {r} corner is a racetrack again"
    );
    // And it is drawn as a curve, not as one flat cut: the deepest point
    // of the arc has to sit clear of the chord across it by more than the
    // rail is wide, or the rail follows a straight line round the bend.
    let sag = r * (1.0 - std::f32::consts::FRAC_1_SQRT_2);
    assert!(
        sag > RAIL_WIDTH * 0.5,
        "the arc sags {sag} across a {RAIL_WIDTH} rail — that is a chamfer"
    );
}

/// A mat's corner and rim are lengths now, so they can be checked against
/// the mat the layout actually hands out.
///
/// The shallowest one there is: [`crate::layout::POD_DEPTH`] plus
/// [`MAT_MARGIN`] on either side. The bound is taken against the playing
/// extent alone anyway, because that is the stricter of the two
/// rectangles — but it is a choice now rather than the only rectangle
/// this crate can reach, which is what it was while the border was
/// `ZONE_MARGIN` in the renderer and no constant here. Both sides again,
/// and each is a mistake that has
/// been made on this table: too round and a board reads as a button, too
/// square and the mat has spikes at the corners where the rim doubles
/// back on itself.
#[test]
fn a_mat_is_a_playing_surface_and_not_a_button() {
    let depth = crate::layout::POD_DEPTH;
    assert!(
        MAT_CORNER < depth * 0.05,
        "a {MAT_CORNER} corner on a mat {depth} deep is a lozenge"
    );
    // The rim has to survive a lane: a border as deep as the row of
    // creatures behind it is a frame, not an edge. The two bounds that
    // need no layout at all — a corner wider than the rim, a rim thick
    // enough to carry a hue — are `const _` assertions beside the
    // constants themselves, where they fail at compile time.
    // The rim has to survive a **lane**, and a lane is what the mat has
    // left once the ledge has taken its share — measuring against the
    // whole depth would let the rim grow every time the shelf did.
    let lane = (depth - MAT_LEDGE) / 3.0;
    assert!(
        MAT_RIM < lane * 0.15,
        "a {MAT_RIM} rim against a {lane} lane is a frame"
    );
    // That the ledge clears the rim, and that it stays a shelf rather
    // than becoming a fourth lane, are both `const _` assertions beside
    // `MAT_LEDGE` itself — `layout` is the same crate, so the card the
    // second one measures against is reachable at compile time.
}

/// Lane dividers read as fine markings, not soft bands across the mat.
#[test]
fn lane_seams_are_fine_but_visible() {
    for height in [256, 512] {
        for ledge_outer in [false, true] {
            let mat = seat_mat(64, height, 0.02, 0.01, [1.0; 3], ledge_outer, false);
            let first = if ledge_outer { MARGIN_FRAC } else { LEDGE_FRAC };
            for lane in [1.0, 2.0] {
                let centre = ((first + COMBAT_FRAC + LANE_FRAC * lane) * height as f32) as u32;
                let bright = (centre - 10..=centre + 10)
                    .filter(|&y| mat.pixel(32, y)[3] > MAT_LANES[0] + MAT_SEAM * 0.5)
                    .count();
                assert!(
                    (1..=height as usize / 128).contains(&bright),
                    "{bright} bright pixels at height {height}, outer ledge {ledge_outer}"
                );
            }
        }
    }
}

/// The ink on the ledge has to be readable *as composited*, which is a
/// different question from how bright the veil is.
///
/// The veil is an alpha on white and the felt underneath it is not the
/// `FELT_CLOTH` constant — the shader's lamp and the sky's tint are both
/// multiplies this module cannot see. So the ground is the same one every
/// other composited bound here uses: a **screenshot**, read off a live
/// duel. Reasoning about the constant instead gives a felt three times
/// too bright and a contrast that agrees with nothing on screen.
///
/// Bounded on both sides for the reason
/// `the_felt_is_dark_enough_to_read_cards_against` is: a one-sided check
/// only stops the mistake it was written after. Too dark and the bar's
/// secondary glyphs go under 4.5:1; too bright and the shelf has become
/// the loudest band on a seat's ground, which is a panel, which is what
/// the ledge exists not to be.
#[test]
fn the_ledge_is_dark_enough_to_read_ink_against() {
    // Bare felt beside a mat, measured off `off_a.png` at a duel framing.
    const FELT_ON_SCREEN: [f32; 3] = [21.0, 63.0, 40.0];
    // `PARCHMENT` #E0D4B0 and `PARCHMENT_EDGE` #AA986E — the bar's
    // numerals and its glyphs, the second being the worst case. Both came
    // down when the UI's parchment took the card shader's value; they
    // measure 7.9 and 4.1 against the 9.1 and 4.7 they had.
    const PARCHMENT: [f32; 3] = [224.0, 212.0, 176.0];
    const PARCHMENT_EDGE: [f32; 3] = [171.0, 152.0, 110.0];

    let ledge = over([1.0, 1.0, 1.0, MAT_LEDGE_VALUE], FELT_ON_SCREEN);
    // Two floors, because the bar writes two kinds of thing on this
    // shelf. Its numerals and its name are text and take WCAG's 4.5:1
    // for text, with room to spare; its glyphs are 10 px marks, which is
    // a graphical object and takes 3:1. They measure 8.7 and 4.5, so the
    // bounds are where the *categories* put them rather than a hair under
    // what today's numbers happen to be.
    for (what, ink, floor) in [
        ("parchment", PARCHMENT, 7.0),
        ("its glyphs", PARCHMENT_EDGE, 3.0),
    ] {
        let ratio = contrast(display_luma(ink), display_luma(ledge));
        assert!(
            ratio >= floor,
            "{what} on the ledge is {ratio:.2}:1 — the seat bar is not \
                 readable on its own shelf"
        );
    }
    // That the shelf is quieter than the quietest lane and louder than
    // bare felt — one shade below the mat rather than a hole cut through
    // it — needs no compositing and is a `const _` beside the constant.
}

/// WCAG contrast between two colours given as 0–255 display triples.
fn display_luma(rgb: [f32; 3]) -> f32 {
    luma([to_linear(rgb[0]), to_linear(rgb[1]), to_linear(rgb[2]), 1.0])
}

fn contrast(a: f32, b: f32) -> f32 {
    let (hi, lo) = if a > b { (a, b) } else { (b, a) };
    (hi + 0.05) / (lo + 0.05)
}

#[test]
fn the_wash_never_speaks_in_a_colour_of_the_pie() {
    // The two signals on this table that a player reads without words:
    // the pie says *whose* and *which colour*, the wash says *where in
    // the turn*. They may not be the same colour. This was broken from
    // the day both existed — ember sat 1.9 degrees from the pie's red and
    // cool 7.1 from its blue — and it went unseen because the wash was a
    // small pool over a dark middle. It stops being survivable when the
    // wash fills the middle and runs up against the mat rims.
    //
    // Fifteen degrees is not a round number chosen in advance: the
    // colours were placed first and the worst surviving pair measures
    // 20.6, so this is the bound with a little air under it.
    const APART: f32 = 15.0;
    for (wname, wash) in WASHES {
        let (wh, ws) = hue_sat(wash);
        if ws < HAS_A_HUE {
            continue;
        }
        for (i, pie) in PIE.iter().enumerate() {
            let (ph, ps) = hue_sat(*pie);
            if ps < HAS_A_HUE {
                continue;
            }
            let gap = hue_gap(wh, ph);
            assert!(
                gap >= APART,
                "{wname} {wash:?} is {gap:.1}° from pie colour {i} {pie:?} — \
                     the turn and the colour wheel would be saying the same thing"
            );
        }
    }
}

#[test]
fn no_two_washes_read_as_the_same_light() {
    // Separating the wash from the pie is only half of it: four washes
    // that collapsed into two would leave a player unable to tell combat
    // from a main phase. A pair may share a hue *only* if one of them is
    // obviously the paler — which is the licence candlelight needs and
    // the only one it gets, because it is the absence of a wash rather
    // than a signal of its own.
    const BY_HUE: f32 = 25.0;
    const BY_SATURATION: f32 = 0.30;
    for (i, (aname, a)) in WASHES.iter().enumerate() {
        for (bname, b) in WASHES.iter().skip(i + 1) {
            let ((ah, asat), (bh, bsat)) = (hue_sat(*a), hue_sat(*b));
            let (hue, sat) = (hue_gap(ah, bh), (asat - bsat).abs());
            assert!(
                hue >= BY_HUE || sat >= BY_SATURATION,
                "{aname} and {bname} are {hue:.1}° apart at {sat:.2} of saturation — \
                     two steps of the turn that look alike"
            );
        }
    }
}

#[test]
fn the_table_looks_the_same_to_everyone_at_it() {
    // No clock, no rng: two runs are the same table. Players screenshot
    // these, and a grain that moved would be the first thing anyone
    // noticed.
    assert_eq!(felt(64).rgba, felt(64).rgba);
    assert_eq!(glow(32).rgba, glow(32).rgba);
}

#[test]
fn the_felt_is_dark_enough_to_read_cards_against() {
    let cloth = felt(96);
    assert_eq!(cloth.rgba.len(), 96 * 96 * 4);
    let mut brightest = 0.0_f32;
    for (_, _, px) in pixels(&cloth) {
        let luma = px[1].mul_add(0.72, px[0].mul_add(0.21, px[2] * 0.07));
        brightest = brightest.max(luma);
        assert!((px[3] - 1.0).abs() < 1e-6, "the felt is opaque");
    }
    assert!(
        brightest < 0.30,
        "a card has to be the brightest thing on the table, not the felt ({brightest})"
    );
    // And the other end of it, which is the half this test was missing
    // the first time the table was drawn: the felt shipped at a third of
    // this and read on screen as a hole in the world. A one-sided bound
    // on "dark enough" is how that passed every run.
    assert!(
        brightest > 0.16,
        "the felt has to be visible cloth, not a black hole ({brightest})"
    );
}

#[test]
fn the_felt_falls_into_shadow_at_its_edges() {
    let cloth = felt(128);
    let luma = |x: u32, y: u32| {
        let p = cloth.pixel(x, y);
        p[1].mul_add(0.72, p[0].mul_add(0.21, p[2] * 0.07))
    };
    let middle = luma(64, 64);
    let corner = luma(2, 2);
    assert!(
        middle > corner * 1.5,
        "the middle of the table should be the lit part: {middle} vs {corner}"
    );
}

/// A seat colour with all three channels far apart, so a test can tell
/// which of them a pixel actually got.
const ACCENT: [f32; 3] = [0.90, 0.20, 0.05];

#[test]
fn only_the_rim_of_a_mat_carries_the_seats_colour() {
    let mat = seat_mat(128, 64, 0.18, 0.05, ACCENT, false, false);
    // The field is the seat's *ground*, not the seat's colour: it stays
    // white so the material's neutral brightness leaves it felt, and a
    // player reads a coloured border around their board rather than a
    // solid sheet of gold with their cards lying on it.
    let field = mat.pixel(64, 32);
    for (channel, got) in field.iter().take(3).enumerate() {
        assert!(
            (got - 1.0).abs() < 1e-3,
            "channel {channel} of the field is {got}, not white"
        );
    }
    // And the rim is the accent — stated as "nearer the accent than the
    // white it is mixed with" rather than as an equality, because the
    // rim is a gradient and no single pixel of it is the pure colour.
    // The property that matters is that a seat is *nameable* from across
    // the table, and a mix that landed on white's side of halfway would
    // not be.
    let rim = mat.pixel(64, 0);
    for (channel, want) in ACCENT.iter().enumerate() {
        let got = rim[channel];
        let halfway = f32::midpoint(1.0, *want);
        assert!(
            got < halfway,
            "channel {channel} of the rim is {got}, nearer white than \
                 the accent's {want}"
        );
    }
    // The accent's own shape survives the mix: this seat's colour is
    // red-dominant and must not come back grey.
    assert!(
        rim[0] > rim[1] && rim[1] > rim[2],
        "the rim lost the accent's ordering: {rim:?}"
    );
}

#[test]
fn a_seat_mat_is_a_rounded_rectangle_with_a_rim() {
    let mat = seat_mat(128, 64, 0.18, 0.05, ACCENT, false, false);
    // Corners are cut away, so a mat never reads as a plain box.
    assert!(mat.pixel(0, 0)[3] < 1e-6, "the corner is rounded off");
    assert!(mat.pixel(127, 63)[3] < 1e-6, "and so is the opposite one");
    // The rim is brighter than the field it encloses.
    let edge = mat.pixel(64, 1)[3];
    let field = mat.pixel(64, 32)[3];
    assert!(
        edge > field,
        "the rim should draw the zone's boundary: {edge} vs {field}"
    );
    assert!(field > 0.0, "the mat itself is visible, not just its rim");
}

#[test]
fn a_seat_mat_shows_where_its_lanes_are() {
    // Tall for the reason `the_mat_fences_its_bands_where_the_layout_put_them`
    // gives below, which this test was the one to be *caught* by: it read
    // 96 rows while `MAT_SEAM_WIDTH` was 0.014, so a hairline was 1.3 rows
    // and the shelf's fence came out one level of 255 above a lane seam.
    // Halving the width to 0.0075 took the fence to `max(h · w, 1.0)` — a
    // single row, the same row the seam gets — and the two arrived at
    // 12/255 apiece. Nothing about the mat had stopped being true; the
    // measurement had stopped being able to see it, which is worth one
    // line of comment because the first instinct was to loosen the claim.
    const H: u32 = 512;
    let mat = seat_mat(256, H, 0.1, 0.03, ACCENT, false, false);
    // The seam belongs *on* the boundary between two lanes, not in the
    // middle of one. Drawn mid-lane it splits every row down its own
    // centre and tells a player the opposite of the truth about where
    // their creatures end.
    //
    // The boundaries are computed rather than written down, because the
    // ledge moved them: three lanes share what the shelf leaves, so a
    // test that sampled thirds was reading the middle of the near lane
    // and passing on a mat with no seams drawn at all.
    #[expect(clippy::cast_possible_truncation, reason = "a row of a texture")]
    let row = |v: f32| (v * H as f32) as u32;
    let lanes = LEDGE_FRAC + COMBAT_FRAC;
    let step = LANE_FRAC;
    let seam = mat.pixel(128, row(lanes + step))[3];
    let mid_lane = mat.pixel(128, row(lanes + step * 0.5))[3];
    assert!(
        seam > mid_lane,
        "the lane seam should be visible: {seam} vs {mid_lane}"
    );
    // And it is a hairline: a few rows apart it is already gone.
    let past = mat.pixel(128, row(lanes + step) + 4)[3];
    assert!(
        past < seam,
        "the seam should be a line, not a band: {seam} then {past}"
    );
    // The ledge is a band of its own at the centre-facing edge, quieter
    // than every lane behind it and fenced off by the brightest seam on
    // the mat.
    let ledge = mat.pixel(128, row(lanes * 0.5))[3];
    assert!(
        ledge < mid_lane,
        "the ledge should be dimmer than the creature lane: {ledge} vs \
             {mid_lane}"
    );
    let fence = mat.pixel(128, row(LEDGE_FRAC))[3];
    assert!(
        fence > seam,
        "the ledge's own seam should be the plainer of the two: {fence} \
             vs {seam}"
    );
}

/// The shelf changes ends; the lanes do not.
///
/// A seat across the table has its board drawn upside-down, so its bar
/// goes at the far end of the mat and is above its creatures on the one
/// screen there is. What must *not* move with it is the lane order —
/// creatures nearest the middle of the table at every seat — because
/// that is where the cards stand and no card turns round when the ink
/// does. The two are one flag apart in the shader and it would be a
/// one-character mistake to flip the uv instead, which is exactly what
/// this catches: it reads the same three rows out of both mats and asks
/// that the veils match end to end while the shelf does not.
#[test]
fn a_flipped_shelf_takes_the_other_end_and_leaves_the_lanes_alone() {
    const H: u32 = 96;
    let inner = seat_mat(256, H, 0.1, 0.03, ACCENT, false, false);
    let outer = seat_mat(256, H, 0.1, 0.03, ACCENT, true, false);
    #[expect(clippy::cast_possible_truncation, reason = "a row of a texture")]
    let row = |v: f32| (v * H as f32) as u32;
    let span = 1.0 - LEDGE_FRAC;
    // Where the shelf *ends* rather than what it is veiled with, because
    // a `Texture` is eight bits a channel and the shelf's own veil and
    // the quietest lane's are 0.0060 and 0.0080 — both 2/255, and the
    // same pixel. The fence between the shelf and the lanes is the
    // brightest seam on a mat and lands at 12/255 on a mat this shallow,
    // so *that* is what a test can read: it is at one end of one mat and
    // at the other end of the other. What it may **not** be read against
    // here is a lane seam, which arrives at the same 12: that comparison
    // needs the rows `a_seat_mat_shows_where_its_lanes_are` spends.
    let fence = |mat: &Texture, at: f32| mat.pixel(128, row(at))[3];
    assert!(
        fence(&inner, LEDGE_FRAC) > fence(&inner, span),
        "the unflipped shelf is fenced off at the centre-facing end"
    );
    assert!(
        fence(&outer, span) > fence(&outer, LEDGE_FRAC),
        "the flipped shelf is fenced off at the outer end"
    );
    // And the creature lane is nearest the middle of the table on both,
    // which is the half that must *not* move: on the flipped mat it
    // starts at the very edge, where the unflipped one has its shelf.
    let creature = MARGIN_FRAC + LANE_FRAC * 0.5;
    assert!(
        outer.pixel(128, row(creature))[3] > inner.pixel(128, row(creature))[3],
        "the flipped mat plays creatures where the unflipped one writes"
    );
    // Read outwards, the three lanes never brighten. Stated as "never
    // brighter" with one strict step across the whole run, because two
    // neighbouring lanes are 0.0135 and 0.0105 and eight bits cannot
    // always tell them apart either.
    for (mat, first, name) in [
        (&inner, LEDGE_FRAC, "inner"),
        (&outer, MARGIN_FRAC, "outer"),
    ] {
        let lane =
            |i: f32| mat.pixel(128, row((i + 0.5).mul_add(LANE_FRAC, first + COMBAT_FRAC)))[3];
        let (near, mid, far) = (lane(0.0), lane(1.0), lane(2.0));
        assert!(
            near >= mid && mid >= far && near > far,
            "the {name} mat's lanes should dim outwards: {near}, {mid}, {far}"
        );
    }
}

/// The bands the mat draws are the bands the layout laid out.
///
/// Both halves of this existed and neither could see the other. A mat is
/// banded in fractions of its own depth, and the depth it is *drawn* at
/// is the playing extent plus a printed border only the renderer knew
/// about — so every fraction was taken over a rectangle 18.5% too
/// shallow. The shelf came out 0.46 units from where the geometry had
/// reserved it, with the seat's bar following the geometry faithfully off
/// its own ledge and onto the creature lane; the two lane seams came out
/// 0.06 and 0.25 units from the rows of cards they are there to fence.
/// Nothing said so, because nothing had ever measured the drawn mat
/// against the layout in the same unit.
///
/// This does, and off the **texture** rather than off the constants the
/// texture was built from: a band written down twice is the mistake, so
/// reading the number back out of one of the copies would only ask
/// whether it equalled itself.
#[test]
fn the_mat_fences_its_bands_where_the_layout_put_them() {
    const H: u32 = 512;
    // Tall, because a fence is a hairline: `MAT_SEAM_WIDTH` of a mat
    // 96 rows deep is one row, and one row cannot be told from its
    // neighbour.
    let lane = (crate::layout::POD_DEPTH - MAT_LEDGE - crate::layout::STAGE_STEP) / 3.0;
    #[expect(clippy::cast_possible_truncation, reason = "a row of a texture")]
    let row = |t: f32| (t / MAT_DRAWN_DEPTH * H as f32) as u32;
    // Measured from the mat's centre-facing edge, in table units, which
    // is the unit `SeatSlot::ledge_corners` and `lane_center` answer in.
    // The shelf is the border plus `MAT_LEDGE`, and the three lanes are
    // each exactly `SeatSlot::lane_height` — the same rectangle a card is
    // placed against.
    //
    // Both mats, because the shelf changes ends and the lanes do not: a
    // near seat spends a border *and* a shelf before its creature row
    // starts, a far seat spends only the border and meets its shelf at
    // the other end. Sampling one of them leaves the other's arithmetic
    // held by nothing — and the mutant that reads the lanes' start as a
    // plain `LEDGE_FRAC`-or-zero is algebraically right on the near mat,
    // so it is exactly the one a single-mat test lets through.
    for ledge_outer in [false, true] {
        let mat = seat_mat(256, H, 0.1, 0.02, ACCENT, ledge_outer, false);
        let lanes = MAT_MARGIN + if ledge_outer { 0.0 } else { MAT_LEDGE };
        let fence = if ledge_outer {
            lanes + crate::layout::STAGE_STEP + lane * 3.0
        } else {
            lanes
        };
        for (what, at) in [
            (
                "the first lane seam",
                lanes + crate::layout::STAGE_STEP + lane,
            ),
            (
                "the second lane seam",
                lanes + crate::layout::STAGE_STEP + lane * 2.0,
            ),
            ("the shelf's own fence", fence),
        ] {
            let here = mat.pixel(128, row(at))[3];
            for away in [-12, 12] {
                let there = mat.pixel(128, row(at).saturating_add_signed(away))[3];
                assert!(
                    here > there,
                    "on the {} seat's mat {what} belongs {at} table units \
                         in from the centre-facing edge, which is row {}: it \
                         reads {here} there and {there} a dozen rows {}",
                    if ledge_outer { "far" } else { "near" },
                    row(at),
                    if away < 0 { "before" } else { "after" }
                );
            }
        }
    }
}

#[test]
fn the_glow_fades_to_nothing_at_its_edge() {
    let g = glow(64);
    assert!(g.pixel(32, 32)[3] > 0.9, "brightest in the middle");
    assert!(g.pixel(0, 32)[3] < 1e-6, "and gone by the edge");
    for (_, _, px) in pixels(&g) {
        assert!((px[0] - 1.0).abs() < 1e-6, "white, so it can be tinted");
    }
}

#[test]
fn a_card_shadow_is_densest_where_the_card_touches_the_table() {
    let s = card_shadow(128, 160, 0.15, 0.1);
    assert!(s.pixel(64, 80)[3] > 0.5, "no shadow under the card");
    // Out through the falloff: fading, and monotonically.
    let mut previous = 1.0;
    for y in (0..19).rev() {
        let alpha = s.pixel(64, y)[3];
        assert!(
            alpha <= previous,
            "the falloff runs the wrong way at row {y}"
        );
        previous = alpha;
    }
}

/// A quad whose texture is still dark at its own edge draws as a square,
/// and a square shadow under a rounded card is the most visible bug there
/// is.
#[test]
fn a_card_shadow_reaches_nothing_before_its_own_edge() {
    let s = card_shadow(128, 160, 0.15, 0.1);
    for x in 0..128 {
        assert!(
            s.pixel(x, 0)[3] < 1e-6 && s.pixel(x, 159)[3] < 1e-6,
            "the shadow is still dark at column {x} of its own edge"
        );
    }
    for y in 0..160 {
        assert!(s.pixel(0, y)[3] < 1e-6 && s.pixel(127, y)[3] < 1e-6);
    }
}

#[test]
fn the_hearth_is_brightest_in_the_middle_and_gone_at_its_edge() {
    let h = hearth(256, HEARTH_INNER, HEARTH_OUTER);
    // Faint on purpose: the pool is half what it shipped as, because at
    // 0.12 the candle warmth over `felt`'s green made the middle of the
    // table read olive. What is asserted is that the lamp is *there*.
    assert!(h.pixel(128, 128)[3] > 0.03, "no light in the middle");
    for x in 0..256 {
        assert!(h.pixel(x, 0)[3] < 1e-6, "still lit at the texture's edge");
        assert!(h.pixel(0, x)[3] < 1e-6);
    }
}

#[test]
fn the_arcane_ring_is_where_it_was_asked_for() {
    let h = hearth(512, HEARTH_INNER, HEARTH_OUTER);
    // Sampled *between* two ticks: on a tick every radius in the band is
    // bright, and the hairline would have nothing to stand out from.
    let at = |r: f32| {
        let angle = (360.0 / f32::from(HEARTH_TICKS) / 2.0).to_radians();
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let (x, y) = (
            (256.0 + r * 256.0 * angle.cos()) as u32,
            (256.0 + r * 256.0 * angle.sin()) as u32,
        );
        h.pixel(x.min(511), y.min(511))[3]
    };
    // The hairline is a local maximum, which is what a line *is*. There
    // is one, on the outer edge: two of them with ticks in between drew
    // three concentric circles and read as a dial.
    let on = at(HEARTH_OUTER);
    assert!(
        on > at(HEARTH_OUTER - 0.04) && on > at(HEARTH_OUTER + 0.04),
        "no hairline at the ring's edge: {} / {on} / {}",
        at(HEARTH_OUTER - 0.04),
        at(HEARTH_OUTER + 0.04)
    );
    // And the band inside it is lifted above the felt outside it, at
    // radii where the pool alone would have it the other way round.
    let (inside, outside) = (at(HEARTH_INNER + 0.04), at(HEARTH_OUTER + 0.1));
    assert!(
        inside > outside,
        "the band is not there: {inside} vs {outside}"
    );
}

#[test]
fn the_ring_carries_its_ticks_all_the_way_round() {
    let h = hearth(512, HEARTH_INNER, HEARTH_OUTER);
    let mid = f32::midpoint(HEARTH_INNER, HEARTH_OUTER);
    let at = |degrees: f32| {
        let angle = degrees.to_radians();
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let (x, y) = (
            (256.0 + mid * 256.0 * angle.cos()) as u32,
            (256.0 + mid * 256.0 * angle.sin()) as u32,
        );
        h.pixel(x.min(511), y.min(511))[3]
    };
    // Eight of them — a compass, not a clock — so every forty-fifth
    // degree is a mark and the angle halfway between two is not.
    let step = 360.0 / f32::from(HEARTH_TICKS);
    for mark in 0..HEARTH_TICKS {
        let on = at(f32::from(mark) * step);
        let off = at(f32::from(mark).mul_add(step, step * 0.5));
        assert!(on > off, "tick {mark} is missing: {on} vs {off}");
    }
}

/// The pool is candlelight and the inlay is gilt. Neither may go blue:
/// a cold light over a green table makes every card's colour identity a
/// guess, which is the one thing the whole unlit design exists to avoid.
#[test]
fn nothing_in_the_hearth_is_a_cold_colour() {
    for (_, _, px) in pixels(&hearth(64, HEARTH_INNER, HEARTH_OUTER)) {
        if px[3] < 1e-6 {
            continue;
        }
        assert!(px[0] >= px[1] && px[1] >= px[2], "a cold pixel: {px:?}");
    }
}

#[test]
fn a_card_shadow_is_black_so_it_only_ever_darkens_the_table() {
    for (_, _, px) in pixels(&card_shadow(32, 40, 0.15, 0.1)) {
        assert!(px[0] < 1e-6 && px[1] < 1e-6 && px[2] < 1e-6);
        assert!(px[3] <= 0.56, "dense enough to read as a hole in the table");
    }
}

/// Combat is the loudest thing the table says, and damage is its peak.
/// A rise that did not peak there would be decoration rather than a
/// warning.
#[test]
fn the_light_rises_into_combat_and_peaks_at_damage() {
    use baylee_view::Step;
    let energy = |step| phase_light(step).energy;
    assert!(energy(Step::Main) < energy(Step::CombatBegin));
    assert!(energy(Step::CombatBegin) < energy(Step::DeclareAttackers));
    assert!(energy(Step::DeclareAttackers) < energy(Step::DeclareBlockers));
    assert!(energy(Step::DeclareBlockers) < energy(Step::CombatDamage));
    assert!(
        (energy(Step::CombatDamage) - 1.0).abs() < f32::EPSILON,
        "damage is the top of the scale, so nothing above it is wasted"
    );
    assert!(energy(Step::CombatEnd) < energy(Step::DeclareAttackers));
}

/// A main phase is where a player reads their own board, so the table
/// stays the colour it was generated as and gets out of the way. The
/// wash is not switched off — that would make the main phase a visible
/// gap in the arc — but it is the lamp's own colour and nearly nothing.
#[test]
fn a_main_phase_barely_washes_at_all() {
    let main = phase_light(baylee_view::Step::Main);
    assert!(main.energy < 0.2, "not {}", main.energy);
    assert!(
        main.rgb[0] > main.rgb[2],
        "still lamplight, not a colour laid over it"
    );
}

/// Every step has a light and none of them is out of range. The match is
/// exhaustive by construction; this is about the numbers in it, which are
/// hand-written and easy to fat-finger.
#[test]
fn every_step_is_lit_and_in_range() {
    use baylee_view::Step;
    for step in [
        Step::Untap,
        Step::Upkeep,
        Step::Draw,
        Step::Main,
        Step::CombatBegin,
        Step::DeclareAttackers,
        Step::DeclareBlockers,
        Step::CombatDamageFirst,
        Step::CombatDamage,
        Step::CombatEnd,
        Step::End,
        Step::Cleanup,
    ] {
        let light = phase_light(step);
        assert!(
            (0.0..=1.0).contains(&light.energy),
            "{step:?} has energy {}",
            light.energy
        );
        for channel in light.rgb {
            assert!((0.0..=1.0).contains(&channel), "{step:?} is out of gamut");
        }
        // Desaturated on purpose: these are lamps over a table, and a
        // fully saturated wash over card art makes colour identity harder
        // to read, which is the one thing the table must not do.
        let low = light.rgb.iter().copied().fold(f32::MAX, f32::min);
        let high = light.rgb.iter().copied().fold(0.0_f32, f32::max);
        assert!(
            high - low < 0.8,
            "{step:?} is a filter, not a lamp: {:?}",
            light.rgb
        );
    }
}

/// The beginning and the end of a turn must not read as the same moment.
#[test]
fn the_turn_does_not_start_and_finish_in_one_colour() {
    let dawn = phase_light(baylee_view::Step::Upkeep).rgb;
    let dusk = phase_light(baylee_view::Step::End).rgb;
    let apart: f32 = (0..3).map(|i| (dawn[i] - dusk[i]).abs()).sum();
    assert!(apart > 0.3, "{dawn:?} and {dusk:?} are the same lamp");
}

/// Parchment is a sheet, not a rectangle of paint.
///
/// Bounded on both sides, which is the lesson the felt taught: it was
/// authored four times too dark and a one-sided "dark enough" assertion
/// let it through. A sheet has to be light enough to take ink and not so
/// light that it glares beside the cards, it has to lose the light at its
/// rim, and the grain has to be there without being a pattern.
#[test]
fn a_sheet_of_parchment_is_warm_lit_and_grained() {
    let sheet = parchment(256);
    let middle = sheet.pixel(128, 128);
    assert!(
        (0.72..0.94).contains(&middle[0]),
        "the middle of the sheet is {middle:?}, which is not parchment"
    );
    assert!(
        middle[0] > middle[1] && middle[1] > middle[2],
        "the sheet is {middle:?} — parchment is warm, and that is not"
    );
    assert!(
        (middle[3] - 1.0).abs() < 1e-6,
        "the sheet is {:.2} opaque, and a question read through a card is \
             a question nobody answers",
        middle[3]
    );

    let rim = sheet.pixel(2, 128);
    // Both ends of it, and both were paid for. The upper bound is the
    // original: a sheet that goes dark at its edge is a hole, not paper.
    // The lower one was 0.05 and had to come down, because that much
    // fall-off is *visible as a seam* — a UI image covers a node's
    // content box and not its padding, so the darkening showed up as a
    // rectangle inside the slip. It still has to be there: a sheet with
    // no edge at all reads as a flat fill.
    assert!(
        rim[0] < middle[0] - 0.02 && rim[0] > middle[0] - 0.30,
        "the rim is {:.3} against the middle's {:.3} — a sheet loses the \
             light at its edge, and only a little",
        rim[0],
        middle[0]
    );

    // Grain: neighbouring pixels differ, and never by so much that the
    // sheet reads as noise.
    let mut most: f32 = 0.0;
    let mut moved = 0_u32;
    for x in 100..156 {
        let step = (sheet.pixel(x, 128)[0] - sheet.pixel(x + 1, 128)[0]).abs();
        most = most.max(step);
        moved += u32::from(step > 1.0 / 255.0);
    }
    assert!(
        moved > 20,
        "only {moved} of 56 steps across the sheet moved"
    );
    assert!(most < 0.06, "the grain jumps {most:.3} between neighbours");
}

/// The gain a held chair's rim is drawn with averages **exactly** one.
///
/// That is the whole design of the mark and not a nicety. A rim drawn
/// quieter would land inside the ladder `table::zone_brightness` already
/// spends on standing — 0.22 for a seat that has lost, 0.62 for one
/// waiting — so "nobody is sitting here" would be read as "this seat is
/// losing interest". Spending the same light differently is the one way
/// to say *different* without saying *quieter*.
///
/// The exactness is what picking a cosine bought: a whole number of
/// periods of `cos` sums to zero, so this needs no tolerance to argue
/// about. Sampled at cell centres, which is where a texel is.
#[test]
fn the_gain_on_a_held_rim_averages_one() {
    const N: u32 = 2400;
    let gains: Vec<f32> = (0..N)
        .map(|k| rim_dash((f32::from(u16::try_from(k).expect("small")) + 0.5) / N as f32))
        .collect();
    let mean = gains.iter().sum::<f32>() / N as f32;
    assert!(
        (mean - 1.0).abs() < 1e-4,
        "a held rim is spending {mean} of the light an unheld one spends"
    );

    // And it is a swing rather than a wobble: the depth is what makes it
    // legible at all, and a gain that averaged one by never leaving it
    // would pass the assertion above and draw nothing.
    let lo = gains.iter().copied().fold(f32::MAX, f32::min);
    let hi = gains.iter().copied().fold(f32::MIN, f32::max);
    assert!(
        lo < 1.0 - MAT_DASH_DEPTH * 0.99 && hi > 1.0 + MAT_DASH_DEPTH * 0.99,
        "the gain runs {lo}..{hi}, not the {MAT_DASH_DEPTH} either side it claims"
    );

    // The pattern closes. `turn` wraps at 1 and `atan2` puts that seam on
    // the mat's right-hand edge, so a fractional `MAT_DASH_COUNT` would
    // leave one dash cut short there — visible, and on the one part of
    // the rim two seats' mats are nearest each other.
    assert!(
        (rim_dash(0.0) - rim_dash(1.0)).abs() < 1e-5,
        "the dashes do not meet where the rim's ends do"
    );
}

/// A held chair's rim is dashed, and the dashes cost it no brightness.
///
/// Two halves, and each is the other's counter. Along a straight run of
/// rim an ordinary mat is **flat to the last bit** — that is the "where
/// the other has none" half, and it is a zero that could perfectly well
/// have been something else. The held mat swings across the same run,
/// symmetrically about the value the plain one holds, which is the "same
/// brightness" half where it is measurable pixel against pixel.
///
/// Over the whole mat the net comes to 1.3% of the light the gain moves
/// about, not 0%, and that is geometry rather than slop: `turn` is an
/// angle normalised by the mat's aspect and not an arc length, so the
/// corners weigh a little differently from the sides. The bound is that
/// measurement with room, and it is far under the depth — a gain that had
/// stopped averaging one would show here as tens of percent.
#[test]
fn a_held_chair_s_rim_is_dashed_rather_than_dimmed() {
    const W: u32 = 512;
    const H: u32 = 196;
    let plain = seat_mat(W, H, 0.1, 0.03, ACCENT, false, false);
    let held = seat_mat(W, H, 0.1, 0.03, ACCENT, false, true);

    let (mut net, mut swing) = (0.0f64, 0.0f64);
    for (x, y, _) in pixels(&plain) {
        let d = f64::from(held.pixel(x, y)[3] - plain.pixel(x, y)[3]);
        net += d;
        swing += d.abs();
    }
    assert!(
        swing > 100.0,
        "the mark moves almost no light at all: {swing}"
    );
    assert!(
        (net / swing).abs() < 0.03,
        "the dash spends {net} net of the {swing} it moves, which is a dimmer"
    );

    // A run along the top edge with no corner in it, two rows in from the
    // outside so the feather is not what is being measured.
    let along = |t: &Texture| {
        let v: Vec<f32> = (60..(W - 60)).map(|x| t.pixel(x, 2)[3]).collect();
        (
            v.iter().copied().fold(f32::MAX, f32::min),
            v.iter().copied().fold(f32::MIN, f32::max),
        )
    };
    let (plain_lo, plain_hi) = along(&plain);
    let (held_lo, held_hi) = along(&held);
    assert!(
        (plain_hi - plain_lo) < 1e-6,
        "an ordinary rim already varies by {} along a straight edge, so a \
             dash cannot be told from it",
        plain_hi - plain_lo
    );
    assert!(
        held_lo < plain_lo * (1.0 - MAT_DASH_DEPTH * 0.5),
        "the deepest gap draws {held_lo} against the {plain_lo} beside it"
    );
    // Symmetric about the rim it replaces: this is the same claim as the
    // net above, made where one pixel can be held against one pixel. Not
    // exact, because the sampled columns do not land on the cosine's own
    // extremes.
    let midpoint = f64::from(held_hi + held_lo) / f64::from(2.0 * plain_lo);
    assert!(
        (midpoint - 1.0).abs() < 0.02,
        "the gain is centred at {midpoint} of the rim, not on it"
    );
}
