//! Elvish Reclaimer — {G} — Creature — Elf Warrior
//! Oracle: This creature gets +2/+2 as long as there are three or more land cards in your graveyard.
//! Oracle: {2}, {T}, Sacrifice a land: Search your library for a land card, put it onto the battlefield tapped, then shuffle.
//! Set: M20 #169 — Core Set 2020 | Scryfall ID: 39c431d7-d94b-46c4-bb89-f3db56214ab4 | Oracle ID: 94f01534-7bb3-4b1e-8f77-e84156408686
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ELVISH_RECLAIMER,
    oracle_id = "94f01534-7bb3-4b1e-8f77-e84156408686",
    scryfall_id = "39c431d7-d94b-46c4-bb89-f3db56214ab4",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Elvish Reclaimer",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::WARRIOR],
        power = Some(1),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
