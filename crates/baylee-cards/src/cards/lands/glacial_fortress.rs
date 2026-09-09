//! Glacial Fortress — (no cost) — Land
//! Oracle: Glacial Fortress enters the battlefield tapped unless you control a PLAINS or an ISLAND.
//! {T}: Add White or Blue.
//! Set: XLN #251 — Ixalan | Scryfall ID: d673a2d5-0c61-48dc-8c8d-06f0c7b6b8bf | Oracle ID: 027dd013-baa7-4111-b3c9-f4d1414e9c45
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
    index: 59,
    oracle_id: "027dd013-baa7-4111-b3c9-f4d1414e9c45",
    scryfall_id: "d673a2d5-0c61-48dc-8c8d-06f0c7b6b8bf",
    faces: &[face! {
        name: "Glacial Fortress",
        types: TypeSet::LAND,
        enter_modifiers: &[EnterModifier::TappedUnless(&CHECK)],
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana_choice(&[ManaColor::White, ManaColor::Blue])])],
}
