//! Burgeoning — {G} — Enchantment
//! Oracle: Whenever an opponent plays a land, you may put a land card from your hand onto the battlefield.
//! Set: C16 #143 — Commander 2016 | Scryfall ID: 6da045f8-6278-4c84-9d39-025adf0789c1 | Oracle ID: 88cf4c1a-5863-45fc-b76d-7ad33b175b2b
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BURGEONING,
    oracle_id = "88cf4c1a-5863-45fc-b76d-7ad33b175b2b",
    scryfall_id = "6da045f8-6278-4c84-9d39-025adf0789c1",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Burgeoning",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::PlaysLand(PlayerRel::EachOpponent),
        &[Effect::PutFromHandOntoBattlefield {
            tapped: false,
            filter: &Filter::LAND,
            mana_value: None,
            optional: true,
        }],
    )],
);
