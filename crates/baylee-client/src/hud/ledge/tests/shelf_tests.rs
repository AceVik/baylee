//! The shelf: what it says, how it is lit, what it reserves, its strips.

#[allow(clippy::wildcard_imports)] // the tests' shared fixtures
use super::*;

/// The shelf is a **dialog**, and nothing on it is borrowed from the
/// parchment or from the table.
///
/// Its own scan rather than an addition to `sheet.rs`'s, which is bound
/// to `sheet.rs` and `overlay.rs` by name and would never have looked at
/// a new file. Three things are forbidden and each for its own reason:
///
/// - `palette::ACCENT` is the teal `docs/design.md` §1.2 retires. Candle
///   is what replaces it, and a shelf that kept one teal control would be
///   the retirement half-done in the most visible place there is.
/// - `BRASS` as a **letter** is gilt, which on this client means a thing
///   already *taken* — an armed deed, a place in an ordering. The shelf
///   asks; it does not report. (As a fill it never appears here at all,
///   so the needle is the `TextColor`.)
/// - `PARCHMENT` in any form is the sheet a question was written on, and
///   §3.1 is the whole argument for why it is not written on one now.
#[test]
fn the_ledge_speaks_only_dialog() {
    for forbidden in [
        "palette::ACCENT",
        "TextColor(palette::BRASS",
        "palette::PARCHMENT",
        // The four the mana pool brought with it out of the chip it was.
        // `ACTIVE` is the one §3.2 names with this very pip as its
        // example — brass is a light at the edge of a *card* — and the
        // other three are the cool near-black panel register the whole
        // dialog ground replaces. Written as `palette::INK)` because
        // `palette::INK` is a prefix of `INK_DANGER` and `INK_BRASS`.
        "palette::ACTIVE",
        "palette::PANEL",
        "palette::MUTED",
        "palette::DEAD",
        "palette::INK)",
    ] {
        assert!(
            !drawn().contains(forbidden),
            "`{forbidden}` is on the shelf, which is a dialog: the ledge \
             has one register and this is not in it"
        );
    }
}

/// The half of this file that draws, which is what a scan is about.
///
/// Everything below `#[cfg(test)]` is these tests, and the needles they
/// name are written out here in full — a scan over the whole file finds
/// its own list and reports the rule as the violation. Cutting at the
/// attribute is the shortest honest answer; the alternative is counting
/// occurrences, which passes the moment a second one appears in a doc
/// comment.
///
/// The drawer is read with it (§10.2 step 6), which is why this hands
/// back a `String` rather than the `&'static str` it used to: the drawer
/// is the same surface under the same rules, and a scan that read only
/// the half of it standing on the shelf would be `sheet.rs`'s file-bound
/// scan made twice. It carries no `#[cfg(test)]` of its own — what it
/// draws is asserted by driving it, in `overlay.rs`' harness — so it
/// joins whole.
///
/// The pool joined it in step 6b, on the same argument and on the day it
/// still *passed*: its three inks are `DIALOG_SOFT`, `LEDGE_DEAD` and
/// `DIALOG`, every one of them a pair this module already measures. A scan
/// is worth widening while it is green — widening one to make a failure go
/// away is how a bound gets loosened to fit what it found.
fn drawn() -> String {
    let shelf = SOURCE
        .split_once("#[cfg(test)]")
        .expect("the tests are still where they were")
        .0;
    format!(
        "{shelf}{}{}",
        include_str!("../drawer.rs"),
        include_str!("../pool.rs")
    )
}

/// What an answer is written in has to be readable on what it is written
/// on.
///
/// The pair the shelf lives or dies by: `DIALOG` on `CANDLE` is the ink
/// of every default button, and the *inverse* — light ink on the candle —
/// is 1.86 : 1 and is forbidden outright by §3.2. A test on the ratio
/// alone would pass on either, so both ends are stated.
///
/// `LEDGE_DEAD / DIALOG` is the third pair, and it is the one bounded on
/// **both** sides on purpose: §3.2 puts it at 3.08 : 1, over the 3.0 a
/// large glyph is held to and under the 4.5 prose needs, because an empty
/// pool's em dash has to be legible without being something to attend to.
/// A one-sided assertion here would let it drift up into the register of
/// the things that are actually there.
#[test]
fn a_candle_is_dark_enough_to_write_on() {
    fn linear(c: f32) -> f32 {
        if c <= 0.040_45 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }
    fn contrast(a: Color, b: Color) -> f32 {
        let luma = |c: Color| {
            let s = c.to_srgba();
            0.2126f32.mul_add(
                linear(s.red),
                0.7152f32.mul_add(linear(s.green), 0.0722 * linear(s.blue)),
            )
        };
        let (one, two) = (luma(a), luma(b));
        (one.max(two) + 0.05) / (one.min(two) + 0.05)
    }

    let ink = contrast(palette::DIALOG, palette::CANDLE);
    assert!(
        ink >= 4.5,
        "the candle's own label has to be readable: {ink:.2}:1"
    );
    // The armed concession is the second loud button and carries the same
    // dark ink, which is what makes the pair one register rather than two
    // colours that happen to be bright. §3.2 measures it at 6.11 : 1.
    let danger = contrast(palette::DIALOG, palette::DANGER);
    assert!(
        danger >= 4.5,
        "the loudest thing the shelf says has to be readable while it \
         says it: {danger:.2}:1"
    );
    let wrong = contrast(palette::DIALOG_INK, palette::CANDLE);
    assert!(
        wrong < 3.0,
        "this test's premise is that light ink on a candle cannot be \
         read, and it measured {wrong:.2}:1"
    );
    // The keycap on a secondary answer: a dark key on the shelf's own
    // ground, and the quiet legend still has to carry 11-point text.
    let legend = contrast(palette::DIALOG_SOFT, palette::DIALOG);
    assert!(
        legend >= 4.5,
        "a keycap nobody can read is a key nobody presses: {legend:.2}:1"
    );
    // The em dash of an empty pool, and a draw offer the engine would
    // refuse: a thing that is not there.
    let absent = contrast(palette::LEDGE_DEAD, palette::DIALOG);
    assert!(
        (3.0..4.5).contains(&absent),
        "`LEDGE_DEAD` says \"nothing here\" and has to be read without \
         being read *at*: {absent:.2}:1 is outside 3.0 … 4.5"
    );
}

/// Which row of the drawer is taken is said by its border, and the wash is
/// only allowed to not get in the way.
///
/// §5 named [`drawer::PICKED_WASH`] and nothing held it to anything, which
/// is the shape of claim this file measures rather than believes. It is
/// bounded on **both** sides, and the two bounds point opposite ways: the
/// ink has to survive the wash (the row still carries words), and the wash
/// must not be credited with the claim it cannot make — 1.08 : 1 against
/// the fill every other row already has is not a difference, so a drawer
/// that lost the candle border would go on passing a test that only asked
/// whether the picked row was washed.
#[test]
fn a_picked_row_is_said_by_its_border() {
    /// sRGB → linear, as in `a_candle_is_dark_enough_to_write_on`.
    fn linear(c: f32) -> f32 {
        if c <= 0.040_45 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }
    fn contrast(a: Color, b: Color) -> f32 {
        let luma = |c: Color| {
            let s = c.to_srgba();
            0.2126f32.mul_add(
                linear(s.red),
                0.7152f32.mul_add(linear(s.green), 0.0722 * linear(s.blue)),
            )
        };
        let (one, two) = (luma(a), luma(b));
        (one.max(two) + 0.05) / (one.min(two) + 0.05)
    }
    /// `over` at its own alpha, laid on an opaque `under`.
    fn over(over: Color, under: Color) -> Color {
        let (o, u) = (over.to_srgba(), under.to_srgba());
        let mix = |a: f32, b: f32| a.mul_add(o.alpha, b * (1.0 - o.alpha));
        Color::srgb(
            mix(o.red, u.red),
            mix(o.green, u.green),
            mix(o.blue, u.blue),
        )
    }

    let taken = over(
        palette::CANDLE.with_alpha(drawer::PICKED_WASH),
        palette::DOCK_GROUND,
    );
    let (fill, edge, ink) = drawer::PANEL_KEY;

    let words = contrast(ink, taken);
    assert!(
        words >= 4.5,
        "a row that has been picked is still a row to read: {words:.2}:1"
    );
    let said = contrast(palette::CANDLE, edge);
    assert!(
        said >= 3.0,
        "the border is the whole of what says a row is taken: {said:.2}:1"
    );
    let alone = contrast(taken, fill);
    assert!(
        alone < 1.2,
        "this test's premise is that the wash cannot say it on its own, \
         and it measured {alone:.2}:1 against an untaken row's fill"
    );
}

/// The shelf does not follow the pointer, and the struct is where that is
/// decided.
///
/// [`LedgeRevision`] exists *because* [`super::HudRevision`] counts the
/// hover; a `hovered` field here would put the shelf back in the rebuild
/// it was taken out of, and every `Feel` on it back to rest whenever the
/// pointer crossed a hand card. Nothing about that is visible at the call
/// site — the field would simply be compared and assigned like any other
/// — so it is read out of the source, the way `hud/tests.rs` reads
/// `HudRevision`'s own fields.
#[test]
fn the_shelf_does_not_follow_the_pointer() {
    let source = SOURCE;
    let body = source
        .split_once("pub struct LedgeRevision {")
        .expect("the struct is still called that")
        .1;
    let body = body.split_once("\n}").expect("and still closes").0;
    let fields: Vec<&str> = body
        .lines()
        .filter_map(|line| {
            let name = line.trim().strip_prefix("pub(super) ")?;
            let (name, _) = name.split_once(':')?;
            name.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
                .then_some(name)
        })
        .collect();
    assert!(fields.len() > 8, "the fields did not parse: {fields:?}");
    for field in fields {
        assert!(
            !field.contains("hover"),
            "`{field}` puts the shelf back in the hover's rebuild, which \
             is the one thing this counter exists to keep it out of"
        );
    }
}

/// A question that has to give something up gives up its keycaps before
/// its words, and its words before its buttons.
///
/// The renderer's half of `client-core`'s ladder: `arrange` decides the
/// rung and this is what the rung is spent on. `Split` is the rung that
/// sends the sentence to the drawer, and until there is a drawer it draws
/// what `Compact` draws — a question the player has already read is worth
/// less than the buttons, which is why `Split` gives it up, but dropping
/// it on the floor instead of putting it somewhere is a different trade.
#[test]
fn the_rungs_are_spent_on_the_caps_first() {
    use baylee_client_core::ledge::Density;
    assert!(Density::Full.shows_keycaps() && Density::Full.shows_sentence());
    assert!(!Density::Compact.shows_keycaps() && Density::Compact.shows_sentence());
    assert!(!Density::Split.shows_keycaps() && !Density::Split.shows_sentence());
    // And the one deviation, stated where it can be found again.
    assert!(
        SOURCE.contains("|| arrangement.density == "),
        "`Split` no longer borrows `Compact`'s sentence: either the \
         drawer exists and this line should go, or the question is being \
         dropped"
    );
}

/// A keycap is a floor and not a width.
///
/// `F6` sits in the 20-pixel square [`KEYCAP_SIDE`] gives it and
/// `Shift+Tab` does not, which is the whole reason `sheet::cap`'s box
/// grows with its legend — and the reason §2.3's own worked example is 18
/// pixels light: it measures `[Space]` at the square, and "Space" is five
/// characters.
#[test]
fn a_long_chord_gets_a_wider_key() {
    let square = CAP_PT * KEYCAP_SIDE;
    assert!(
        (cap_width("6") - square).abs() < f32::EPSILON,
        "one character sits in the square: {}",
        cap_width("6")
    );
    for chord in ["Shift+Tab", "Space"] {
        assert!(
            cap_width(chord) > square,
            "`{chord}` does not, and clipping a chord would be worse than \
             growing its key: {}",
            cap_width(chord)
        );
    }
    // And the slip in §2.3's own worked example, written down where it
    // can be checked: it measures `[Space]` at the square, which is 18
    // pixels light. The priority middle is 606 and not 588 — still
    // `Full` at 1280 against the design's own 325 and 214, so nothing
    // downstream moved when it was found. (Both of those are measured
    // numbers now, 365 and 222, and 623 is still `Full`.)
    assert!(
        cap_width("Space") - square > 17.0,
        "the design's arithmetic was out by {}",
        cap_width("Space") - square
    );
}

/// The two strips are one shape, and the *only* thing that differs is
/// which end they hang off.
///
/// The owner asked for the mana pool as the tray — *"Es soll symetrisch
/// zum Tray aussehen nur auf der linken Seite"* — and since #264 the
/// players' strip and the pool are the pair it holds. Symmetry is the
/// kind of claim that is true on the day it is typed and quietly stops
/// being true afterwards: a strip a pixel taller or a corner rounder than
/// its twin reads as a mistake and fails nothing. Both sides of the
/// assertion are needed and they are different assertions. That the four
/// shared numbers agree is what a copied constant would break; that each
/// strip's *own* side is at [`EDGE`] and its other side is `Auto` is what
/// a strip drawn from the wrong variant would break, and the first would
/// pass right through it.
#[test]
fn the_two_strips_are_one_shape_hung_off_two_ends() {
    let left = strip_node(StripSide::Left);
    let right = strip_node(StripSide::Right);

    assert_eq!(left.height, right.height, "one strip is taller");
    assert_eq!(left.bottom, right.bottom, "one strip sits deeper");
    assert_eq!(left.padding, right.padding, "one strip is packed tighter");
    assert_eq!(left.border, right.border, "one strip is outlined harder");
    assert_eq!(
        left.border_radius, right.border_radius,
        "one strip is rounder"
    );

    assert_eq!(
        left.left,
        px(EDGE),
        "the players are not against their margin"
    );
    assert_eq!(left.right, Val::Auto, "the players are pinned at both ends");
    assert_eq!(right.right, px(EDGE), "the pool is not against its margin");
    assert_eq!(right.left, Val::Auto, "the pool is pinned at both ends");
}

/// The left column reserves at least what it draws, in either language.
///
/// [`TOOLS_WIDE`] and [`TOOLS_NARROW`] are measured in the running client
/// and a measurement cannot be re-taken here, so this is the bound either
/// side of it: a **floor** from the row's own arithmetic, because
/// under-reserving is the direction that costs something — `arrange`
/// slides the question to clear what it is told the neighbours take, so a
/// row wider than it says crowds the question by the difference — and a
/// ceiling, because a floor alone is passed by any number large enough
/// and the whole point of this change was that the left column got
/// smaller.
///
/// The floor deliberately leaves the mark out. [`super::text_width`] is
/// an estimator for the *text* faces and knows nothing about the icon
/// font's advances, so counting a guess at them would put a guess on the
/// strict side of an assertion. What is left is still the larger half and
/// still moves with every edit that matters: another button, a longer
/// label, a bigger [`TOOL_PT`].
#[test]
#[allow(clippy::cast_precision_loss)] // five buttons, counted
fn the_hand_tools_reserve_at_least_the_row_they_draw() {
    use crate::hand_order::HandOrder;
    use baylee_client_core::i18n::Lang;

    // What one button costs before its mark and its word: two borders,
    // the air either side, and the step from the mark to the word.
    let furniture = 2.0 + 2.0 * TOOL_PAD_X + TOOL_MARK_GAP;

    for lang in [Lang::De, Lang::En] {
        let wide: f32 = HandOrder::ALL
            .iter()
            .map(|order| furniture + super::text_width(order.label(lang), TOOL_PT, true))
            .sum::<f32>()
            + TOOL_GAP * (HandOrder::ALL.len() - 1) as f32;
        assert!(
            TOOLS_WIDE >= wide,
            "{lang:?}: five sorting buttons draw at least {wide:.1} and \
             {TOOLS_WIDE} is reserved for them, so the question is \
             crowded by the difference"
        );
        assert!(
            TOOLS_WIDE <= wide * 1.7,
            "{lang:?}: {TOOLS_WIDE} is reserved against a row of at most \
             {wide:.1} plus its marks — a reservation that generous is \
             not a measurement any more"
        );

        // And the narrow row, whose one button wears the longest of the
        // five labels inside `Hand: … ›`.
        let narrow = HandOrder::ALL
            .iter()
            .map(|order| {
                furniture
                    + super::text_width(
                        &format!("Hand: {} \u{203a}", order.label(lang)),
                        TOOL_PT,
                        true,
                    )
            })
            .fold(0.0_f32, f32::max);
        assert!(
            TOOLS_NARROW >= narrow,
            "{lang:?}: the cycling button draws at least {narrow:.1} and \
             {TOOLS_NARROW} is reserved for it"
        );
        assert!(
            TOOLS_NARROW <= narrow * 1.7,
            "{lang:?}: {TOOLS_NARROW} is reserved against a button of at \
             most {narrow:.1} plus its mark"
        );
    }
}
