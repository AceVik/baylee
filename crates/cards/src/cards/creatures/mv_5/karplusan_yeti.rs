//! Karplusan Yeti — {3}{R}{R} — Creature — Yeti
//! Oracle: {T}: This creature deals damage equal to its power to target creature. That creature deals damage equal to its power to this creature.
//! Set: 9ED #198 — Ninth Edition | Scryfall ID: 6ad95118-104c-41d4-a393-941e7b97fd8e | Oracle ID: d6d84884-4a58-4187-aa5c-20b39b379590

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KARPLUSAN_YETI,
    oracle_id = "d6d84884-4a58-4187-aa5c-20b39b379590",
    scryfall_id = "6ad95118-104c-41d4-a393-941e7b97fd8e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Karplusan Yeti",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::YETI],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[
            Effect::DamageEqualToPower {
                dealer: TargetSlot::This,
                to: TargetSlot::First,
            },
            Effect::DamageEqualToPower {
                dealer: TargetSlot::First,
                to: TargetSlot::This,
            },
        ],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
