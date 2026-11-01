//! Repercussion — {1}{R}{R} — Enchantment
//! Oracle: Whenever a creature is dealt damage, this enchantment deals that much damage to that creature's controller.
//! Set: UDS #95 — Urza's Destiny | Scryfall ID: d0f3c78e-16c0-4fbc-8ef4-fbf610f9d464 | Oracle ID: f6069a4f-744e-43e8-9f8b-7c13b8b0187f

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REPERCUSSION,
    oracle_id = "f6069a4f-744e-43e8-9f8b-7c13b8b0187f",
    scryfall_id = "d0f3c78e-16c0-4fbc-8ef4-fbf610f9d464",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Repercussion",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::DealtDamage(&Filter::CREATURE),
        &[Effect::DealDamage {
            amount: Amount::EventAmount,
            target: TargetSpec::Player(PlayerRel::ControllerOfEvent),
        }],
    )],
);
