//! Fireball — {X}{R} — Sorcery
//! Oracle: This spell costs {1} more to cast for each target beyond the first.
//! Oracle: Fireball deals X damage divided evenly, rounded down, among any number of targets.
//! Set: CLB #175 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: df45a43e-a5b7-4fd4-873b-7b3c021be198 | Oracle ID: aa7714b0-2bfb-458a-8ebf-37ec2c53383e
// IMPLEMENTED — extra target costs and even damage division among legal targets.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FIREBALL,
    oracle_id = "aa7714b0-2bfb-458a-8ebf-37ec2c53383e",
    scryfall_id = "df45a43e-a5b7-4fd4-873b-7b3c021be198",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Fireball",
        mana_cost = mana!("{X}{R}"),
        types = TypeSet::SORCERY,
        extra_target_cost = 1,
    ),],
    abilities = &[spell!(
        &[Effect::DealDamageEvenly {
            amount: Amount::X,
            target: TargetSpec::AnyTarget,
        }],
        targets = Some(TargetReq::up_to(TargetSpec::AnyTarget, 255)),
    )],
);
