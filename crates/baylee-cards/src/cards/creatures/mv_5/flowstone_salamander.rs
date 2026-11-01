//! Flowstone Salamander — {3}{R}{R} — Creature — Salamander
//! Oracle: {R}: This creature deals 1 damage to target creature blocking it.
//! Set: TMP #175 — Tempest | Scryfall ID: bf5b6749-42a6-498b-8908-b28d1749dea6 | Oracle ID: a0e40eb1-a909-405b-8fb4-3759df8edf7f
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const BLOCKING_THIS: Filter = Filter::And(&[Filter::CREATURE, Filter::BlockingSource]);

card!(
    index = index::FLOWSTONE_SALAMANDER,
    oracle_id = "a0e40eb1-a909-405b-8fb4-3759df8edf7f",
    scryfall_id = "bf5b6749-42a6-498b-8908-b28d1749dea6",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Flowstone Salamander",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SALAMANDER],
        power = Some(3),
        toughness = Some(4),
    ),],
    abilities = &[activated!(
        cost!("{R}"),
        &[Effect::DealDamage {
            target: TargetSpec::Object(&BLOCKING_THIS),
            amount: Amount::Fixed(1),
        }],
        target = Some(TargetSpec::Object(&BLOCKING_THIS)),
    )],
);
