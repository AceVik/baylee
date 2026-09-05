//! Captivating Cave — (no cost) — Land — Cave
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Add one mana of any color.
//! Oracle: {4}, {T}, Sacrifice this land: Put two +1/+1 counters on target creature. Activate only as a sorcery.
//! Set: LCI #268 — The Lost Caverns of Ixalan | Scryfall ID: 1d1a645e-85c7-4044-b817-6e24744d245e | Oracle ID: 4c77767a-8133-43bc-b7a5-09a73259d354
// IMPLEMENTED — land with three activated abilities: {T} → {C}; {1}{T} → any
// color; {4}{T} + sacrifice self → two +1/+1 counters on target creature
// (sorcery speed).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 329,
    oracle_id: "4c77767a-8133-43bc-b7a5-09a73259d354",
    scryfall_id: "1d1a645e-85c7-4044-b817-6e24744d245e",
    faces: &[
    face! {
        name: "Captivating Cave",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::CAVE],
    },
    ],
    coverage: Coverage::Implemented,
    abilities: &[
        // {T}: Add {C}.
        mana_ability!(Cost::TAP, &[Effect::mana(ManaColor::Colorless, 1)]),
        // {1}, {T}: Add one mana of any color.
        mana_ability!(
            Cost { mana: baylee_core::mana!("{1}"), parts: &[CostPart::TapSelf] },
            &[Effect::mana_of_any_color()]
        ),
        // {4}, {T}, Sacrifice this land: Put two +1/+1 counters on target
        // creature. Activate only as a sorcery.
        activated!(
            Cost {
                mana: baylee_core::mana!("{4}"),
                parts: &[CostPart::TapSelf, CostPart::SacrificeSelf],
            },
            &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(2),
            }],
            target: Some(TargetSpec::Object(&Filter::CREATURE)),
            timing: ActivationTiming::SorcerySpeed
        ),
    ],
}
