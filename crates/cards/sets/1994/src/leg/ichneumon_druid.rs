//! Ichneumon Druid — {1}{G}{G} — Creature — Human Druid
//! Oracle: Whenever an opponent casts an instant spell other than the first instant spell that player casts each turn, this creature deals 4 damage to that player.
//! Set: LEG #191 — Legends | Scryfall ID: cf2313bb-6f9b-49d6-b069-9f3b77b6e107 | Oracle ID: 6aba2a04-0117-429e-a5df-6ba4645129d8

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ICHNEUMON_DRUID,
    oracle_id = "6aba2a04-0117-429e-a5df-6ba4645129d8",
    scryfall_id = "cf2313bb-6f9b-49d6-b069-9f3b77b6e107",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Ichneumon Druid",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::DRUID],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::InstantCastAfterFirst {
            who: PlayerRel::Opponent
        },
        &[Effect::DealDamage {
            amount: Amount::Fixed(4),
            target: TargetSpec::Player(PlayerRel::EventPlayer)
        }],
    ),],
);
