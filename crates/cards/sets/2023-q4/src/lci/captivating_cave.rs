//! Captivating Cave — (no cost) — Land — Cave
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Add one mana of any color.
//! Oracle: {4}, {T}, Sacrifice this land: Put two +1/+1 counters on target creature. Activate only as a sorcery.
//! Set: LCI #268 — The Lost Caverns of Ixalan | Scryfall ID: 1d1a645e-85c7-4044-b817-6e24744d245e | Oracle ID: 4c77767a-8133-43bc-b7a5-09a73259d354
// IMPLEMENTED — {C}, then any color for {1}, then a sorcery-speed
// {4}, {T}, Sacrifice this land for two +1/+1 counters on a target creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CAPTIVATING_CAVE,
    oracle_id = "4c77767a-8133-43bc-b7a5-09a73259d354",
    scryfall_id = "1d1a645e-85c7-4044-b817-6e24744d245e",
    faces = &[face!(
        name = "Captivating Cave",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::CAVE],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(cost!("{1}", TapSelf), &[Effect::mana_of_any_color()]),
        activated!(
            cost!("{4}", TapSelf, SacrificeSelf),
            &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(2),
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
