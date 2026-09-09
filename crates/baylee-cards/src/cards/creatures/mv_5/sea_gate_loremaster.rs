//! Sea Gate Loremaster — {4}{U} — Creature — Merfolk Wizard Ally
//! Oracle: {T}: Draw a card for each Ally you control.
//! Set: ZEN #63 — Zendikar | Scryfall ID: 5cd723c8-4b3d-4fbb-a825-79934279382d | Oracle ID: 6eed122b-9760-47fd-8ba2-adeda8054e0d
// IMPLEMENTED — tap to draw per Ally.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static ALLY_YOU: Filter =
    Filter::And(&[Filter::ControlledByYou, Filter::HasSubtype(creature::ALLY)]);

card! {
    index: 141,
    oracle_id: "6eed122b-9760-47fd-8ba2-adeda8054e0d",
    scryfall_id: "5cd723c8-4b3d-4fbb-a825-79934279382d",
    faces: &[face! {
        name: "Sea Gate Loremaster",
        mana_cost: baylee_core::mana!("{4}{U}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::MERFOLK, creature::WIZARD, creature::ALLY],
        power: Some(1),
        toughness: Some(3),
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[activated!(Cost::TAP, &[Effect::DrawCards {
            amount: Amount::CountOf {
                filter: &ALLY_YOU,
                zone: ZoneSel::Battlefield,
            },
        }])],
}
