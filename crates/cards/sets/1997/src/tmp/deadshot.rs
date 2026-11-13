//! Deadshot — {3}{R} — Sorcery
//! Oracle: Tap target creature. It deals damage equal to its power to another target creature.
//! Set: TPR #129 — Tempest Remastered | Scryfall ID: ad82258b-3856-49ee-9acd-926feb15e070 | Oracle ID: 711c10cb-7a85-417d-be24-79d4e4a89993
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

const ANOTHER_TARGET_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::Reference)]);

card!(
    index = index::DEADSHOT,
    oracle_id = "711c10cb-7a85-417d-be24-79d4e4a89993",
    scryfall_id = "ad82258b-3856-49ee-9acd-926feb15e070",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Deadshot",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[
            Effect::TapTarget,
            Effect::DamageEqualToPower {
                dealer: TargetSlot::First,
                to: TargetSlot::Second,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets = Some(TargetReq::one(TargetSpec::Object(&ANOTHER_TARGET_CREATURE))),
    )],
);
