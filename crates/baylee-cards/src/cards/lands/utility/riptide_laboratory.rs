//! Riptide Laboratory — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}{U}, {T}: Return target Wizard you control to its owner's hand.
//! Set: C14 #305 — Commander 2014 | Scryfall ID: 25a9cb87-e572-4885-8561-1d4b158ec7e4 | Oracle ID: 444d50dd-a44a-42db-bbf6-d0978e3bd6a3
// IMPLEMENTED.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

/// "Target Wizard **you control**" — the card returns your own creature to
/// save it from removal, and returning an opponent's Wizard would be a
/// different card entirely.
static WIZARD: Filter = Filter::And(&[
    Filter::HasSubtype(creature::WIZARD),
    Filter::ControlledByYou,
]);

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
        activated!(Cost { mana: baylee_core::mana!("{1}{U}"), parts: &[CostPart::TapSelf] }, &[Effect::ReturnToHand {
                target: TargetSpec::Object(&WIZARD),
            }], target: Some(TargetSpec::Object(&WIZARD))),
    ],
}
