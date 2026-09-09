//! Heliod's Intervention — {X}{W}{W} — Instant
//! Oracle: Choose one —
//! Oracle: • Destroy X target artifacts and/or enchantments.
//! Oracle: • Target player gains twice X life.
//! Set: OTC #81 — Outlaws of Thunder Junction Commander | Scryfall ID: 9519bb3a-bed3-48e8-93ae-9e9b2e7d646a | Oracle ID: e7564d66-767c-4cd9-a5f0-0f2488a4a74b
// IMPLEMENTED — both modes (X-target destroy / 2X lifegain).

static ARTIFACT_OR_ENCHANTMENT: Filter = Filter::Or(&[Filter::ARTIFACT, Filter::ENCHANTMENT]);

use baylee_cards_dsl::prelude::*;

card! {
    index: 67,
    oracle_id: "e7564d66-767c-4cd9-a5f0-0f2488a4a74b",
    scryfall_id: "9519bb3a-bed3-48e8-93ae-9e9b2e7d646a",
    faces: &[face! {
        name: "Heliod's Intervention",
        mana_cost: baylee_core::mana!("{X}{W}{W}"),
        types: TypeSet::INSTANT,
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::ModalSpell {
        modes: &[
            mode!(&[Effect::Destroy {
                    target: TargetSpec::Object(&ARTIFACT_OR_ENCHANTMENT),
                }], target: Some(TargetSpec::Object(&ARTIFACT_OR_ENCHANTMENT))),
            mode!(&[Effect::GainLifeFor {
                    amount: Amount::DoubleX,
                    who: PlayerRel::Chosen,
                }], target: Some(TargetSpec::AnyPlayer)),
        ],
    }],
}
