//! Blazing Effigy — {1}{R} — Creature — Elemental
//! Oracle: When this creature dies, it deals X damage to target creature, where X is 3 plus the amount of damage dealt to this creature this turn by other sources named Blazing Effigy.
//! Set: LEG #134 — Legends | Scryfall ID: 921011ff-1696-4575-9198-abe993a0ee7a | Oracle ID: 7879b37c-5e2a-4945-9378-47744b715a6c
// IMPLEMENTED — the dies trigger deals 3 plus the damage other sources named
// Blazing Effigy dealt to it this turn to a target creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "Other sources named Blazing Effigy", read as they were when they dealt it.
static OTHER_EFFIGY: Filter = Filter::And(&[Filter::Named("Blazing Effigy"), Filter::Another]);

card!(
    index = index::BLAZING_EFFIGY,
    oracle_id = "7879b37c-5e2a-4945-9378-47744b715a6c",
    scryfall_id = "921011ff-1696-4575-9198-abe993a0ee7a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Blazing Effigy",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(0),
        toughness = Some(3),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::DealDamage {
            amount: Amount::Plus {
                base: &Amount::DamageThisTurn {
                    to: DamageTo::Source,
                    by: &OTHER_EFFIGY,
                    combat: None,
                },
                offset: 3,
            },
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )],
);
