//! Diminishing Returns — {2}{U}{U} — Sorcery
//! Oracle: Each player shuffles their hand and graveyard into their library. You exile the top ten cards of your library. Then each player draws up to seven cards.
//! Set: EMA #46 — Eternal Masters | Scryfall ID: f9ed455f-95cd-48ca-8b0d-4db259347bb6 | Oracle ID: f9b02a7c-c122-4282-90e2-81fcfa663193
// NOT SUPPORTED: exiling the top ten cards of your library before drawing and choosing to draw fewer than seven cards are not supported.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DIMINISHING_RETURNS,
    oracle_id = "f9b02a7c-c122-4282-90e2-81fcfa663193",
    scryfall_id = "f9ed455f-95cd-48ca-8b0d-4db259347bb6",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "each player shuffles their hand and graveyard into their library and then draws seven cards; exiling the top ten cards of your library in between and letting each player choose to draw fewer than seven cards are not supported",
    ),
    faces = &[face!(
        name = "Diminishing Returns",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[
        Effect::ShuffleIntoLibrary {
            who: PlayerRel::EachPlayer,
            hand: true,
            graveyard: true,
        },
        Effect::DrawCardsFor {
            amount: Amount::Fixed(7),
            who: PlayerRel::EachPlayer,
        },
    ])],
);
