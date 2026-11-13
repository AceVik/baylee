//! Marsh Viper — {3}{G} — Creature — Snake
//! Oracle: Whenever this creature deals damage to a player, that player gets two poison counters. (A player with ten or more poison counters loses the game.)
//! Set: 5ED #315 — Fifth Edition | Scryfall ID: ba4c0606-f9af-4dee-bc36-5051395b5f44 | Oracle ID: 5249df56-7571-46c0-882e-cf88e452fb4d

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MARSH_VIPER,
    oracle_id = "5249df56-7571-46c0-882e-cf88e452fb4d",
    scryfall_id = "ba4c0606-f9af-4dee-bc36-5051395b5f44",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Marsh Viper",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SNAKE],
        power = Some(1),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::DealsDamageToPlayer {
            source: &Filter::This,
            player: PlayerRel::EachPlayer,
        },
        &[Effect::Poison {
            who: PlayerRel::EventPlayer,
            amount: Amount::Fixed(2),
        }],
    )],
);
