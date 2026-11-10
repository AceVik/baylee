//! Fire and Brimstone — {3}{W}{W} — Instant
//! Oracle: Fire and Brimstone deals 4 damage to target player who attacked this turn and 4 damage to you.
//! Set: DRK #9 — The Dark | Scryfall ID: d5208dbb-63d2-4789-8ef9-f82499a43b3a | Oracle ID: b1f857c5-cd22-4a4e-8039-1e7eeb3754d8

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FIRE_AND_BRIMSTONE,
    oracle_id = "b1f857c5-cd22-4a4e-8039-1e7eeb3754d8",
    scryfall_id = "d5208dbb-63d2-4789-8ef9-f82499a43b3a",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Fire and Brimstone",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::DealDamage {
                amount: Amount::Fixed(4),
                target: TargetSpec::PlayerWhoAttackedThisTurn,
            },
            Effect::DealDamage {
                amount: Amount::Fixed(4),
                target: TargetSpec::Player(PlayerRel::You),
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::PlayerWhoAttackedThisTurn))
    )],
);
