//! Order of Yawgmoth — {2}{B}{B} — Creature — Phyrexian Zombie Knight
//! Oracle: Fear (This creature can't be blocked except by artifact creatures and/or black creatures.)
//! Oracle: Whenever this creature deals damage to a player, that player discards a card.
//! Set: USG #144 — Urza's Saga | Scryfall ID: 555e858d-d8d2-4626-9343-070cf86949ab | Oracle ID: c1795e9c-31c9-4892-a796-7656e09257b3
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ORDER_OF_YAWGMOTH,
    oracle_id = "c1795e9c-31c9-4892-a796-7656e09257b3",
    scryfall_id = "555e858d-d8d2-4626-9343-070cf86949ab",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::FEAR,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Order of Yawgmoth",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::PHYREXIAN,
            subtypes::creature::ZOMBIE,
            subtypes::creature::KNIGHT,
        ],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::DealsDamageToPlayer {
            source: &Filter::This,
            player: PlayerRel::EachPlayer,
        },
        &[Effect::DiscardForPlayers {
            who: PlayerRel::DamagedPlayer,
            count: 1,
        }],
    )],
);
