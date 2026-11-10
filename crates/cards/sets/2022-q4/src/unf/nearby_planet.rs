//! Nearby Planet — (no cost) — Land
//! Oracle: Rangeling (This card is every land type, including Plains, Island, Swamp, Mountain, Forest, Desert, Gate, Lair, Locus, and all those Urza's ones.)
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, sacrifice it unless you pay {1}.
//! Set: UNF #198 — Unfinity | Scryfall ID: da785d1b-6b90-4b65-9efb-d7f329405318 | Oracle ID: 19d34126-8266-4da1-b7ef-67ecfa2dbbee

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NEARBY_PLANET,
    oracle_id = "19d34126-8266-4da1-b7ef-67ecfa2dbbee",
    scryfall_id = "da785d1b-6b90-4b65-9efb-d7f329405318",
    color_identity = ColorSet::from_slice(&[
        Color::Black,
        Color::Green,
        Color::Red,
        Color::Blue,
        Color::White
    ]),
    coverage = Coverage::Implemented,
    keywords = KeywordSet::RANGELING,
    faces = &[face!(
        name = "Nearby Planet",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::PlayerMayPayOr {
            player: PlayerRel::You,
            mana: Amount::Fixed(1),
            effect: &Effect::SacrificeSelf,
        }]
    ),],
);
