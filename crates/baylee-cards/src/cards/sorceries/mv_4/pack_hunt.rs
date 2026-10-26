//! Pack Hunt — {3}{G} — Sorcery
//! Oracle: Search your library for up to three cards with the same name as target creature, reveal them, put them into your hand, then shuffle.
//! Set: NEM #109 — Nemesis | Scryfall ID: 1c46caa8-efc0-4b72-b122-61e5d86a5b86 | Oracle ID: 3ee41993-9d14-43a8-bf92-433c4f3aab01
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PACK_HUNT,
    oracle_id = "3ee41993-9d14-43a8-bf92-433c4f3aab01",
    scryfall_id = "1c46caa8-efc0-4b72-b122-61e5d86a5b86",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Pack Hunt",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
