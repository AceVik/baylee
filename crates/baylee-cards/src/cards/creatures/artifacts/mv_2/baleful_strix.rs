//! Baleful Strix — {U}{B} — Artifact Creature — Bird
//! Oracle: Flying, deathtouch
//! Oracle: When this creature enters, draw a card.
//! Set: OTC #215 — Outlaws of Thunder Junction Commander | Scryfall ID: be8439e6-f779-49f0-806a-b04995697a6a | Oracle ID: 37688720-03de-4eca-a82d-a0afe8d58adc
// IMPLEMENTED — keywords + ETB cantrip.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card! {
    index: 10,
    oracle_id: "37688720-03de-4eca-a82d-a0afe8d58adc",
    scryfall_id: "be8439e6-f779-49f0-806a-b04995697a6a",
    faces: &[face! {
        name: "Baleful Strix",
        mana_cost: baylee_core::mana!("{U}{B}"),
        types: TypeSet::CREATURE.union(TypeSet::ARTIFACT),
        subtypes: &[creature::BIRD],
        power: Some(1),
        toughness: Some(1),
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::Black]),
    keywords: KeywordSet::FLYING.union(KeywordSet::DEATHTOUCH),
    coverage: Coverage::Implemented,
    abilities: &[triggered!(Trigger::EntersBattlefield(&Filter::This), &[Effect::DrawCards {
            amount: Amount::Fixed(1),
        }])],
}
