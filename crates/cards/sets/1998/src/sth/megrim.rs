//! Megrim — {2}{B} — Enchantment
//! Oracle: Whenever an opponent discards a card, this enchantment deals 2 damage to that player.
//! Set: M10 #104 — Magic 2010 | Scryfall ID: 3b04fca4-b4de-4343-ba58-d8aea39edc19 | Oracle ID: 633ad9e2-9f55-4a1c-9248-661ad4b0e1dc
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MEGRIM,
    oracle_id = "633ad9e2-9f55-4a1c-9248-661ad4b0e1dc",
    scryfall_id = "3b04fca4-b4de-4343-ba58-d8aea39edc19",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Megrim",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::Discarded {
            who: PlayerRel::EachOpponent,
            filter: &Filter::Any,
        },
        &[Effect::DealDamage {
            target: TargetSpec::Player(PlayerRel::EventPlayer),
            amount: Amount::Fixed(2),
        }],
    )],
);
