//! `cards/lands/cabal_coffers.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Five lands that pour "for each …", and the five different questions that
/// phrase turns out to be.
///
/// `Amount::CountOf` has said this since Gaea's Cradle was written by hand;
/// what was missing was a reader for `SVar:X:Count$Valid …`, so eleven of the
/// pool's stubs were refused for an amount the DSL could already spell. Each
/// row is a filter the count would be wrong without, and every one of them is
/// a *different* wrongness:
///
/// - **Cabal Coffers** counts Swamps you control; a Badlands is a Swamp, so
///   the subtype and not the name is the question.
/// - **Cabal Stronghold** prints `basic` in front of the same word, and the
///   same board therefore answers one instead of two. Nothing but that atom
///   separates the two cards.
/// - **Serra's Sanctum** and **Tolarian Academy** count a card *type*, and
///   both say "you control": the opponent's copy is seated deliberately, and
///   a filter that lost `ControlledByYou` would count it.
/// - **Cloudpost** is the one that says the opposite — "each Locus **on the
///   battlefield**" — and it counts itself and the opponent's. This is the
///   case that would be silently narrowed by the condition vocabulary, where
///   a count naming no player is refused outright.
///
/// The colour is measured as a *delta*, because the fixture that pays the
/// price makes mana too.
#[test]
fn a_land_that_counts_pours_one_mana_for_each_thing_its_own_filter_matches() {
    for (seed, land, color, mine, theirs, generic, index, want) in [
        (
            930,
            cabal_coffers(),
            ManaColor::Black,
            vec![swamp(), badlands()],
            vec![swamp()],
            2,
            0,
            2,
        ),
        (
            931,
            cabal_stronghold(),
            ManaColor::Black,
            vec![swamp(), badlands()],
            vec![swamp()],
            3,
            1,
            1,
        ),
        (
            932,
            serra_s_sanctum(),
            ManaColor::White,
            vec![doubling_season()],
            vec![doubling_season()],
            0,
            0,
            1,
        ),
        (
            933,
            tolarian_academy(),
            ManaColor::Blue,
            vec![lightning_greaves(), lightning_greaves()],
            vec![lightning_greaves()],
            0,
            0,
            2,
        ),
        (
            934,
            cloudpost(),
            ManaColor::Colorless,
            vec![glimmerpost()],
            vec![glimmerpost()],
            0,
            0,
            3,
        ),
    ] {
        one_counted_land(seed, land, color, &mine, &theirs, generic, index, want);
    }
}
