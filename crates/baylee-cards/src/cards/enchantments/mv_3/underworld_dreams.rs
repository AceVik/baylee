//! Underworld Dreams — {B}{B}{B} — Enchantment
//! Oracle: Whenever an opponent draws a card, this enchantment deals 1 damage to that player.
//! Set: THB #121 — Theros Beyond Death | Scryfall ID: 03919c86-1c4a-43b0-a2db-54ca6ae1ac57 | Oracle ID: 967cf377-ae26-464d-85ac-8448b5a911f7
// IMPLEMENTED — every opposing draw pings that opponent for 1.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::UNDERWORLD_DREAMS,
    oracle_id = "967cf377-ae26-464d-85ac-8448b5a911f7",
    scryfall_id = "03919c86-1c4a-43b0-a2db-54ca6ae1ac57",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Underworld Dreams",
        mana_cost = mana!("{B}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::Draws(PlayerRel::Opponent),
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            // "That player": the opponent who drew. Not targeted.
            target: TargetSpec::Player(PlayerRel::EventPlayer),
        }]
    )],
);
