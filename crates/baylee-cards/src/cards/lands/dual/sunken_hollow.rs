//! Sunken Hollow — (no cost) — Land
//! Oracle: Sunken Hollow enters the battlefield tapped unless you control a SWAMP or an FOREST.
//! {T}: Add Black or Green.
//! Set: BFZ #249 — Battle for Zendikar | Scryfall ID: 3a8eef9b-9b03-42cd-a27a-07021bf0b33f | Oracle ID: cd2c90ac-2b04-461c-92f3-939871b6b6a3
// IMPLEMENTED — checkland (ETB tapped unless you control a SWAMP/FOREST) + 2-color mana.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static CHECK: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::LAND,
    Filter::Or(&[
        Filter::HasSubtype(land::SWAMP),
        Filter::HasSubtype(land::FOREST),
    ]),
]);

card! {
    index: 159,
    oracle_id: "cd2c90ac-2b04-461c-92f3-939871b6b6a3",
    scryfall_id: "3a8eef9b-9b03-42cd-a27a-07021bf0b33f",
    faces: &[face! {
        name: "Sunken Hollow",
        types: TypeSet::LAND,
        enter_modifiers: &[EnterModifier::TappedUnless(&CHECK)],
    }],
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Green]),
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana_choice(&[ManaColor::Black, ManaColor::Green])])],
}
