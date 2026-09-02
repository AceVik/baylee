//! Drowned Catacomb — (no cost) — Land
//! Oracle: Drowned Catacomb enters the battlefield tapped unless you control a ISLAND or an SWAMP.
//! {T}: Add Blue or Black.
//! Set: XLN #252 — Ixalan | Scryfall ID: ebea49ab-e5cf-46d9-ae35-226a7321ede0 | Oracle ID: 819fc966-434e-470f-91e9-a38df974ad17
// IMPLEMENTED — checkland (ETB tapped unless you control a ISLAND/SWAMP) + 2-color mana.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static CHECK: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::LAND,
    Filter::Or(&[
        Filter::HasSubtype(land::ISLAND),
        Filter::HasSubtype(land::SWAMP),
    ]),
]);

card! {
    index: 36,
    oracle_id: "819fc966-434e-470f-91e9-a38df974ad17",
    scryfall_id: "ebea49ab-e5cf-46d9-ae35-226a7321ede0",
    faces: &[face! {
        name: "Drowned Catacomb",
        types: TypeSet::LAND,
        enter_modifiers: &[EnterModifier::TappedUnless(&CHECK)],
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana_choice(&[ManaColor::Blue, ManaColor::Black])])],
}
