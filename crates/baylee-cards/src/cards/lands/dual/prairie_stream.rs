//! Prairie Stream — (no cost) — Land
//! Oracle: Prairie Stream enters the battlefield tapped unless you control a PLAINS or an ISLAND.
//! {T}: Add White or Blue.
//! Set: BFZ #241 — Battle for Zendikar | Scryfall ID: b2e133b4-2263-4ac2-8d16-7bf307d5e104 | Oracle ID: 5330e24a-8568-446e-840a-594cd08bd1bc
// IMPLEMENTED — checkland (ETB tapped unless you control a PLAINS/ISLAND) + 2-color mana.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static CHECK: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::LAND,
    Filter::Or(&[
        Filter::HasSubtype(land::PLAINS),
        Filter::HasSubtype(land::ISLAND),
    ]),
]);

card! {
    index: 117,
    oracle_id: "5330e24a-8568-446e-840a-594cd08bd1bc",
    scryfall_id: "b2e133b4-2263-4ac2-8d16-7bf307d5e104",
    faces: &[face! {
        name: "Prairie Stream",
        types: TypeSet::LAND,
        enter_modifiers: &[EnterModifier::TappedUnless(&CHECK)],
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana_choice(&[ManaColor::White, ManaColor::Blue])])],
}
