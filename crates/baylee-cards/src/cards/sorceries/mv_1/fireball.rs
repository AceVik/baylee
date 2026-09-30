//! Fireball — {X}{R} — Sorcery
//! Oracle: This spell costs {1} more to cast for each target beyond the first.
//! Oracle: Fireball deals X damage divided evenly, rounded down, among any number of targets.
//! Set: CLB #175 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: df45a43e-a5b7-4fd4-873b-7b3c021be198 | Oracle ID: aa7714b0-2bfb-458a-8ebf-37ec2c53383e
// PARTIAL — damage divided evenly among any number of targets, and the cost
// of each target beyond the first, are not in the DSL; X damage to one
// target.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FIREBALL,
    oracle_id = "aa7714b0-2bfb-458a-8ebf-37ec2c53383e",
    scryfall_id = "df45a43e-a5b7-4fd4-873b-7b3c021be198",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "damage divided evenly among any number of targets, and the cost of each target beyond the first, are not in the DSL; X damage to one target"
    ),
    faces = &[face!(
        name = "Fireball",
        mana_cost = mana!("{X}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[
        spell!(
            &[Effect::DealDamage {
                amount: Amount::X,
                target: TargetSpec::AnyTarget
            }],
            targets = Some(TargetReq::one(TargetSpec::AnyTarget))
        ),
        // NOT SUPPORTED: This spell costs {1} more to cast for each target beyond the
        // first.
        // NOT SUPPORTED: Fireball deals X damage divided evenly, rounded down, among any
        // number of targets.
    ],
);
