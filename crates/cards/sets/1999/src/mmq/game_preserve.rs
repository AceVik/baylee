//! Game Preserve — {2}{G} — Enchantment
//! Oracle: At the beginning of your upkeep, each player reveals the top card of their library. If all cards revealed this way are creature cards, put those cards onto the battlefield under their owners' control.
//! Set: MMQ #248 — Mercadian Masques | Scryfall ID: 1bb62356-72bb-4dc1-a2f9-45a3aca62e41 | Oracle ID: 409e0a43-0e85-47ea-9d40-de0860b12952
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GAME_PRESERVE,
    oracle_id = "409e0a43-0e85-47ea-9d40-de0860b12952",
    scryfall_id = "1bb62356-72bb-4dc1-a2f9-45a3aca62e41",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Game Preserve",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
