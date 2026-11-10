//! Infernal Contract — {B}{B}{B} — Sorcery
//! Oracle: Draw four cards. You lose half your life, rounded up.
//! Set: 7ED #143 — Seventh Edition | Scryfall ID: f451a70f-1f1c-4fd6-ab0c-4b77a043c324 | Oracle ID: 4372205f-45e6-49c7-b1fd-13a41e20d4de
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::INFERNAL_CONTRACT,
    oracle_id = "4372205f-45e6-49c7-b1fd-13a41e20d4de",
    scryfall_id = "f451a70f-1f1c-4fd6-ab0c-4b77a043c324",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Infernal Contract",
        mana_cost = mana!("{B}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[
        Effect::draw(4),
        Effect::LoseHalfLife {
            player: PlayerRel::You,
        },
    ])],
);
