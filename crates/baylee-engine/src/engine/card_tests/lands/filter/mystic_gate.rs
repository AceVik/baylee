//! `cards/lands/filter/mystic_gate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mystic Gate: `{W/U}, {T}: Add {W}{W}, {W}{U}, or {U}{U}.`
///
/// The first **hybrid activation cost** this pool pays. `{W/U}` is one mana
/// of either colour (CR 107.4e), and the whole point of a filter land is that
/// the price is a colour: it turns one coloured mana into two, so a board
/// that can only make colorless cannot start the engine at all. The card was
/// written with `cost!("{1}", …)` — one *generic* mana, payable by anything —
/// and every other reading of it was right, which is why it stood: the
/// `//! Oracle:` header printed `{W/U}`, the effect made its two combination
/// mana, and only the price was wrong. `xtask validate`'s activation-cost
/// check is the half that says so without a game; this is the half that shows
/// what the difference buys an opponent.
///
/// Both halves are asserted, because only the pair is discriminating:
///
/// - **Colorless is refused.** A second Mystic Gate taps for `{C}`, and that
///   `{C}` cannot pay `{W/U}`. Against `{1}` it paid, and the land filtered
///   for free.
/// - **A Plains pays.** One `{W}` in, two picks out — and the two answers may
///   differ, which is `combination: true` and the reason this land is worth
///   playing over a Plains.
#[test]
fn a_filter_land_charges_a_coloured_mana_and_colorless_will_not_do() {
    // All ten of the hybrid cycle — two written by hand, eight by the
    // transcoder — through one rule, which is the point of holding them
    // together: a generated card is a rule's output and a rule is testable.
    // The basic in each row makes the *first* colour of the pair, so every
    // land is paid with a colour its own price names. That White is in both
    // hand-written pairs is the accident that let `{1}` stand: a board that
    // can pay the real price pays the wrong one too, and only a *colorless*
    // board tells the two apart.
    for (land, basic, colors) in [
        (mystic_gate(), plains(), [ManaColor::White, ManaColor::Blue]),
        (
            fetid_heath(),
            plains(),
            [ManaColor::White, ManaColor::Black],
        ),
        (
            cascade_bluffs(),
            island(),
            [ManaColor::Blue, ManaColor::Red],
        ),
        (
            sunken_ruins(),
            island(),
            [ManaColor::Blue, ManaColor::Black],
        ),
        (
            flooded_grove(),
            forest(),
            [ManaColor::Green, ManaColor::Blue],
        ),
        (
            wooded_bastion(),
            forest(),
            [ManaColor::Green, ManaColor::White],
        ),
        (
            fire_lit_thicket(),
            mountain(),
            [ManaColor::Red, ManaColor::Green],
        ),
        (
            rugged_prairie(),
            mountain(),
            [ManaColor::Red, ManaColor::White],
        ),
        (graven_cairns(), swamp(), [ManaColor::Black, ManaColor::Red]),
        (
            twilight_mire(),
            swamp(),
            [ManaColor::Black, ManaColor::Green],
        ),
    ] {
        one_filter_land(land, basic, colors);
    }
}
