//! Desperate Gambit — {R} — Instant
//! Oracle: Choose a source you control and flip a coin. If you win the flip, the next time that source would deal damage this turn, it deals double that damage instead. If you lose the flip, the next time it would deal damage this turn, prevent that damage.
//! Set: WTH #96 — Weatherlight | Scryfall ID: f4245160-274e-4c39-9bcd-c64e9a44dfdb | Oracle ID: d8328d27-e27f-4c84-8ab5-cabb80a34f54
// NOT SUPPORTED: choosing a source you control before the flip and doubling the next damage it would deal this turn on a won flip (or preventing only the next instance of damage on a lost flip) is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DESPERATE_GAMBIT,
    oracle_id = "d8328d27-e27f-4c84-8ab5-cabb80a34f54",
    scryfall_id = "f4245160-274e-4c39-9bcd-c64e9a44dfdb",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "flips a coin and on a lost flip prevents all damage a chosen source would deal this turn (`ChooseDamageSourcePreventAllThisTurn`); choosing a source you control before the flip and doubling the next damage it would deal this turn on a won flip (or preventing only the next instance of damage on a lost flip) is not yet modeled",
    ),
    faces = &[face!(
        name = "Desperate Gambit",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::FlipCoin {
        won: &[],
        lost: &[Effect::ChooseDamageSourcePreventAllThisTurn],
    }])],
);
