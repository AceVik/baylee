//! Winds of Change — {R} — Sorcery
//! Oracle: Each player shuffles the cards from their hand into their library, then draws that many cards.
//! Set: ME1 #111 — Masters Edition | Scryfall ID: 3f09e393-7318-43eb-95f8-0f2797a771d7 | Oracle ID: f525cf10-e24c-4c46-9a13-6f8579d09d50
// PARTIAL — the hand-shuffle is written; "then draws that many cards" is not.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WINDS_OF_CHANGE,
    oracle_id = "f525cf10-e24c-4c46-9a13-6f8579d09d50",
    scryfall_id = "3f09e393-7318-43eb-95f8-0f2797a771d7",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "no effect draws a number of cards equal to what a player shuffled \
         into their library, and no Amount reads a hand that no longer exists"
    ),
    faces = &[face!(
        name = "Winds of Change",
        mana_cost = mana!("{R}"),
        types = TypeSet::SORCERY,
    ),],
    // NOT SUPPORTED: "then draws that many cards." —
    // `Effect::ShuffleIntoLibrary` moves each hand into its library and
    // shuffles but reports no count, and by the time an `Effect::DrawCardsFor`
    // resolved, the hand it would count is empty. No Amount remembers the
    // cards a resolution shuffled in.
    abilities = &[spell!(&[Effect::ShuffleIntoLibrary {
        who: PlayerRel::EachPlayer,
        hand: true,
        graveyard: false
    }])],
);
