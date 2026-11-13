//! Winds of Change — {R} — Sorcery
//! Oracle: Each player shuffles the cards from their hand into their library, then draws that many cards.
//! Set: ME1 #111 — Masters Edition | Scryfall ID: 3f09e393-7318-43eb-95f8-0f2797a771d7 | Oracle ID: f525cf10-e24c-4c46-9a13-6f8579d09d50
// IMPLEMENTED — each player shuffles their hand into their library, then draws
// as many cards as they shuffled in.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WINDS_OF_CHANGE,
    oracle_id = "f525cf10-e24c-4c46-9a13-6f8579d09d50",
    scryfall_id = "3f09e393-7318-43eb-95f8-0f2797a771d7",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Winds of Change",
        mana_cost = mana!("{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[
        Effect::ShuffleIntoLibrary {
            who: PlayerRel::EachPlayer,
            hand: true,
            graveyard: false
        },
        // "then draws that many cards" — each player draws the number they
        // shuffled in, every player's count read before anyone draws.
        Effect::DrawCardsFor {
            amount: Amount::ThisWayBy {
                what: ThisWay::PutIntoLibraryFromHand,
                who: PlayerRel::Recipient,
            },
            who: PlayerRel::EachPlayer,
        },
    ])],
);
