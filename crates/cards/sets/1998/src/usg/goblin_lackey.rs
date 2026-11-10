//! Goblin Lackey — {R} — Creature — Goblin
//! Oracle: Whenever this creature deals damage to a player, you may put a Goblin permanent card from your hand onto the battlefield.
//! Set: USG #190 — Urza's Saga | Scryfall ID: 9b848caa-aad8-4060-8f86-304a8556de2d | Oracle ID: 213fbbdd-f1bc-4bba-9124-7b26e9a706c8
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GOBLIN_LACKEY,
    oracle_id = "213fbbdd-f1bc-4bba-9124-7b26e9a706c8",
    scryfall_id = "9b848caa-aad8-4060-8f86-304a8556de2d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Goblin Lackey",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::DealsDamageToPlayer {
            source: &Filter::This,
            player: PlayerRel::EachPlayer,
        },
        &[Effect::PutFromHandOntoBattlefield {
            tapped: false,
            filter: &Filter::And(&[
                Filter::HasSubtype(subtypes::creature::GOBLIN),
                Filter::Or(&[
                    Filter::ARTIFACT,
                    Filter::CREATURE,
                    Filter::ENCHANTMENT,
                    Filter::LAND,
                    Filter::PLANESWALKER,
                    Filter::HasType(TypeSet::BATTLE),
                ]),
            ]),
            mana_value: None,
            optional: true,
        }],
    )],
);
