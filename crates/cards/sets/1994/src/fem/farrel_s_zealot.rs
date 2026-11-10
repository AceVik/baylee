//! Farrel's Zealot — {1}{W}{W} — Creature — Human
//! Oracle: Whenever this creature attacks and isn't blocked, you may have it deal 3 damage to target creature. If you do, this creature assigns no combat damage this turn.
//! Set: ME2 #14 — Masters Edition II | Scryfall ID: 5417bcef-5181-4ca9-9391-b10e61ee4c69 | Oracle ID: 0d063c0e-66bc-4a9c-98aa-a381b9cea120
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FARREL_S_ZEALOT,
    oracle_id = "0d063c0e-66bc-4a9c-98aa-a381b9cea120",
    scryfall_id = "5417bcef-5181-4ca9-9391-b10e61ee4c69",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Farrel's Zealot",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::AttacksAndIsntBlocked(&Filter::This),
        &[Effect::MayDo {
            effects: &[
                Effect::DealDamage {
                    amount: Amount::Fixed(3),
                    target: TargetSpec::Object(&Filter::CREATURE),
                },
                Effect::ModifySource {
                    modifier: Modifier::AssignsNoCombatDamage,
                    duration: Duration::UntilEndOfTurn,
                },
            ],
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    ),],
);
