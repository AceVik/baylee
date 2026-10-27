//! Bloodline Shaman — {1}{G} — Creature — Elf Wizard Shaman
//! Oracle: {T}: Choose a creature type. Reveal the top card of your library. If that card is a creature card of the chosen type, put it into your hand. Otherwise, put it into your graveyard.
//! Set: ONS #249 — Onslaught | Scryfall ID: 5fdfc473-8477-4c04-a4e7-ecac1b0a5716 | Oracle ID: a9c2bf1b-2d5e-42c5-bc76-8429aaae161c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BLOODLINE_SHAMAN,
    oracle_id = "a9c2bf1b-2d5e-42c5-bc76-8429aaae161c",
    scryfall_id = "5fdfc473-8477-4c04-a4e7-ecac1b0a5716",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Bloodline Shaman",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::ELF,
            subtypes::creature::WIZARD,
            subtypes::creature::SHAMAN
        ],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
