//! Karakas — (no cost) — Land
//! Oracle: {T}: Add {W}. {T}: Return target legendary creature to its owner\u{2019}s hand.
//! Set: EMA #240 — Eternal Masters | Scryfall ID: e52214e1-404a-405a-b08e-20e13c087338 | Oracle ID: 59119143-c0fa-49dd-adf0-e2fd3029c48b
// IMPLEMENTED.

use baylee_cards_dsl::prelude::*;

static LEGENDARY_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasSupertype(SupertypeSet::LEGENDARY),
]);

card! {
    index: 79,
    oracle_id: "59119143-c0fa-49dd-adf0-e2fd3029c48b",
    scryfall_id: "e52214e1-404a-405a-b08e-20e13c087338",
    faces: &[face! {
        name: "Karakas",
        types: TypeSet::LAND,
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
        activated!(Cost::TAP, &[Effect::ReturnToHand {
                target: TargetSpec::Object(&LEGENDARY_CREATURE),
            }], target: Some(TargetSpec::Object(&LEGENDARY_CREATURE))),
    ],
}
