//! Elkin Lair — {3}{R} — World Enchantment
//! Oracle: At the beginning of each player's upkeep, that player exiles a card at random from their hand. The player may play that card this turn. At the beginning of the next end step, if the player hasn't played the card, they put it into their graveyard.
//! Set: VIS #78 — Visions | Scryfall ID: bcb625ba-3718-4988-962c-bf2e11eb4c16 | Oracle ID: b3488947-7142-4e6c-9c92-2728469b3ad9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ELKIN_LAIR,
    oracle_id = "b3488947-7142-4e6c-9c92-2728469b3ad9",
    scryfall_id = "bcb625ba-3718-4988-962c-bf2e11eb4c16",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Elkin Lair",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
