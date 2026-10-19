//! Dwarven Catapult — {X}{R} — Instant
//! Oracle: Dwarven Catapult deals X damage divided evenly, rounded down, among all creatures target opponent controls.
//! Set: ME1 #91 — Masters Edition | Scryfall ID: 9d6310cd-95e2-4e8c-952f-f7c572726a01 | Oracle ID: 2aae77ed-1a91-41f5-8dc2-e353d63a7586
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DWARVEN_CATAPULT,
    oracle_id = "2aae77ed-1a91-41f5-8dc2-e353d63a7586",
    scryfall_id = "9d6310cd-95e2-4e8c-952f-f7c572726a01",
    coverage = Coverage::Implemented,
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Dwarven Catapult",
        mana_cost = mana!("{X}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::DealDamageEvenlyToFilter {
            amount: Amount::X,
            filter: &Filter::OPPONENT_CREATURE,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    ),],
);
