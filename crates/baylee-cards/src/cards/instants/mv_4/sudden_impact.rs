//! Sudden Impact — {3}{R} — Instant
//! Oracle: Sudden Impact deals damage to target player equal to the number of cards in that player's hand.
//! Set: 10E #241 — Tenth Edition | Scryfall ID: a6560bb7-28d4-4b5c-bdfa-986a5c114349 | Oracle ID: 2bf65e7d-6d31-4ad6-93cd-623612d3175d

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SUDDEN_IMPACT,
    oracle_id = "2bf65e7d-6d31-4ad6-93cd-623612d3175d",
    scryfall_id = "a6560bb7-28d4-4b5c-bdfa-986a5c114349",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Sudden Impact",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::HandSize(PlayerRel::Chosen),
            target: TargetSpec::AnyPlayer,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
    )],
);
