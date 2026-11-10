//! Pit Scorpion — {2}{B} — Creature — Scorpion
//! Oracle: Whenever this creature deals damage to a player, that player gets a poison counter. (A player with ten or more poison counters loses the game.)
//! Set: 5ED #187 — Fifth Edition | Scryfall ID: fe106ff1-cdc6-44cc-adb7-131203a05292 | Oracle ID: 4804fc42-588c-487a-8ff7-bcc16749fc1f

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PIT_SCORPION,
    oracle_id = "4804fc42-588c-487a-8ff7-bcc16749fc1f",
    scryfall_id = "fe106ff1-cdc6-44cc-adb7-131203a05292",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Pit Scorpion",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SCORPION],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::DealsDamageToPlayer {
            source: &Filter::This,
            player: PlayerRel::EachPlayer
        },
        &[Effect::Poison {
            who: PlayerRel::EventPlayer,
            amount: Amount::Fixed(1)
        }]
    )],
);
