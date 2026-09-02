//! Isolated Chapel — (no cost) — Land
//! Oracle: Isolated Chapel enters the battlefield tapped unless you control a PLAINS or an SWAMP.
//! {T}: Add White or Black.
//! Set: XLN #253 — Ixalan | Scryfall ID: 78814c92-b52c-462a-866f-3e7da9db9f70 | Oracle ID: 7e5d9efe-48a9-434b-bb09-056e0e09cc9a
// IMPLEMENTED — checkland (ETB tapped unless you control a PLAINS/SWAMP) + 2-color mana.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static CHECK: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::LAND,
    Filter::Or(&[
        Filter::HasSubtype(land::PLAINS),
        Filter::HasSubtype(land::SWAMP),
    ]),
]);

card! {
    index: 75,
    oracle_id: "7e5d9efe-48a9-434b-bb09-056e0e09cc9a",
    scryfall_id: "78814c92-b52c-462a-866f-3e7da9db9f70",
    faces: &[face! {
        name: "Isolated Chapel",
        types: TypeSet::LAND,
        enter_modifiers: &[EnterModifier::TappedUnless(&CHECK)],
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana_choice(&[ManaColor::White, ManaColor::Black])])],
}
