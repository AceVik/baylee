//! Riptide Laboratory — (no cost) — Land
//! Oracle: {T}: Add {C}. {T}: Return target Wizard to its owner\u{2019}s hand.
//! Set: C14 #305 — Commander 2014 | Scryfall ID: 25a9cb87-e572-4885-8561-1d4b158ec7e4 | Oracle ID: 444d50dd-a44a-42db-bbf6-d0978e3bd6a3
// IMPLEMENTED.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static WIZARD: Filter = Filter::HasSubtype(creature::WIZARD);

card! {
    index: 134,
    oracle_id: "444d50dd-a44a-42db-bbf6-d0978e3bd6a3",
    scryfall_id: "25a9cb87-e572-4885-8561-1d4b158ec7e4",
    faces: &[face! {
        name: "Riptide Laboratory",
        types: TypeSet::LAND,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(Cost::TAP, &[Effect::ReturnToHand {
                target: TargetSpec::Object(&WIZARD),
            }], target: Some(TargetSpec::Object(&WIZARD))),
    ],
}
