//! Retromancer — {2}{R}{R} — Creature — Lizard Shaman
//! Oracle: Whenever this creature becomes the target of a spell or ability, this creature deals 3 damage to that spell or ability's controller.
//! Set: USG #209 — Urza's Saga | Scryfall ID: e9fbf63d-7106-47d5-97c3-4596d8239925 | Oracle ID: 2c34949d-0765-455d-9ef4-81bc162b48d9
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RETROMANCER,
    oracle_id = "2c34949d-0765-455d-9ef4-81bc162b48d9",
    scryfall_id = "e9fbf63d-7106-47d5-97c3-4596d8239925",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Retromancer",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LIZARD, subtypes::creature::SHAMAN],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::BecomesTarget,
        &[Effect::DealDamage {
            amount: Amount::Fixed(3),
            target: TargetSpec::Player(PlayerRel::ControllerOfEvent),
        }],
    ),],
);
