//! Grasping Dunes — (no cost) — Land — Desert
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}, Sacrifice this land: Put a -1/-1 counter on target creature. Activate only as a sorcery.
//! Set: AKH #244 — Amonkhet | Scryfall ID: a8fcc939-6a31-4fb3-abe7-7663b85868dd | Oracle ID: 47d16c11-3033-44f3-9a12-2daf3453cc5b
// IMPLEMENTED — {C} mana, plus a sorcery-speed {1}, {T}, sacrifice activation
// that puts a -1/-1 counter on a target creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GRASPING_DUNES,
    oracle_id = "47d16c11-3033-44f3-9a12-2daf3453cc5b",
    scryfall_id = "a8fcc939-6a31-4fb3-abe7-7663b85868dd",
    faces = &[face!(
        name = "Grasping Dunes",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::DESERT],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{1}", TapSelf, SacrificeSelf),
            &[Effect::AddCounter {
                kind: CounterKind::M1M1,
                amount: Amount::Fixed(1),
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
