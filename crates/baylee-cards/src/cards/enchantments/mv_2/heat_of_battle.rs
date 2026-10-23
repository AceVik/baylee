//! Heat of Battle — {1}{R} — Enchantment
//! Oracle: Whenever a creature blocks, this enchantment deals 1 damage to that creature's controller.
//! Set: STH #88 — Stronghold | Scryfall ID: 8dbb98db-f2ee-446f-9170-dd05b1a7dbd8 | Oracle ID: 400285bb-ded9-4018-a7ea-3e6f4d24b900
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HEAT_OF_BATTLE,
    oracle_id = "400285bb-ded9-4018-a7ea-3e6f4d24b900",
    scryfall_id = "8dbb98db-f2ee-446f-9170-dd05b1a7dbd8",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Heat of Battle",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::Blocks(&Filter::CREATURE),
        &[Effect::DealDamage {
            target: TargetSpec::Player(PlayerRel::ControllerOfEvent),
            amount: Amount::Fixed(1),
        }],
    )],
);
