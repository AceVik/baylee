//! Haunting Echoes — {3}{B}{B} — Sorcery
//! Oracle: Exile all cards from target player's graveyard other than basic land cards. For each card exiled this way, search that player's library for all cards with the same name as that card and exile them. Then that player shuffles.
//! Set: M11 #99 — Magic 2011 | Scryfall ID: 5aa96bc7-b92f-4df5-89b5-0793b9e56509 | Oracle ID: 3abd345a-69a5-486e-ada9-dd1ca0decc28
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HAUNTING_ECHOES,
    oracle_id = "3abd345a-69a5-486e-ada9-dd1ca0decc28",
    scryfall_id = "5aa96bc7-b92f-4df5-89b5-0793b9e56509",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Haunting Echoes",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
