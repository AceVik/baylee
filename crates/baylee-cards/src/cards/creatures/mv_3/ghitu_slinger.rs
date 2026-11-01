//! Ghitu Slinger — {2}{R} — Creature — Human Nomad
//! Oracle: Echo {2}{R} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Oracle: When this creature enters, it deals 2 damage to any target.
//! Set: EMA #133 — Eternal Masters | Scryfall ID: 923de31f-0941-4df1-8939-3eed75b7eb39 | Oracle ID: 8e2b5f90-e405-4b24-a71a-0988c37cf67a
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GHITU_SLINGER,
    oracle_id = "8e2b5f90-e405-4b24-a71a-0988c37cf67a",
    scryfall_id = "923de31f-0941-4df1-8939-3eed75b7eb39",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Ghitu Slinger",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::NOMAD],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[
        AbilityDef::Echo {
            cost: mana!("{2}{R}"),
        },
        triggered!(
            Trigger::ETB,
            &[Effect::DealDamage {
                amount: Amount::Fixed(2),
                target: TargetSpec::AnyTarget,
            }],
            targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
        ),
    ],
);
