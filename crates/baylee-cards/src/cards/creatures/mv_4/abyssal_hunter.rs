//! Abyssal Hunter — {3}{B} — Creature — Human Assassin
//! Oracle: {B}, {T}: Tap target creature. This creature deals damage equal to its power to that creature.
//! Set: 6ED #109 — Classic Sixth Edition | Scryfall ID: 3c3f6ad7-4782-40db-8eb3-e71d71ec3388 | Oracle ID: b96ccc2f-88e5-4522-924e-76fc76dc3b39
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ABYSSAL_HUNTER,
    oracle_id = "b96ccc2f-88e5-4522-924e-76fc76dc3b39",
    scryfall_id = "3c3f6ad7-4782-40db-8eb3-e71d71ec3388",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Abyssal Hunter",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::ASSASSIN],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{B}", TapSelf),
        &[
            Effect::TapTarget,
            Effect::DealDamage {
                target: TargetSpec::Object(&Filter::CREATURE),
                amount: Amount::SourcePower,
            },
        ],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
