//! Zur's Weirding — {3}{U} — Enchantment
//! Oracle: Players play with their hands revealed.
//! Oracle: If a player would draw a card, they reveal it instead. Then any other player may pay 2 life. If a player does, put that card into its owner's graveyard. Otherwise, that player draws a card.
//! Set: 9ED #114 — Ninth Edition | Scryfall ID: 741612e3-fe3a-4099-9d2b-4a654c4ce949 | Oracle ID: c6eaa147-3566-43a9-999a-d58b877496f5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ZUR_S_WEIRDING,
    oracle_id = "c6eaa147-3566-43a9-999a-d58b877496f5",
    scryfall_id = "741612e3-fe3a-4099-9d2b-4a654c4ce949",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Zur's Weirding",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
