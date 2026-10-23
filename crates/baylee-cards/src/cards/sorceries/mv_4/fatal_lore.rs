//! Fatal Lore — {2}{B}{B} — Sorcery
//! Oracle: An opponent chooses one —
//! Oracle: • You draw three cards.
//! Oracle: • You destroy up to two target creatures that player controls. They can't be regenerated. That player draws up to three cards.
//! Set: ALL #48 — Alliances | Scryfall ID: 24ba0b83-9671-4ee7-996d-57a3616b9c66 | Oracle ID: aeb85786-16fc-45d8-9383-60054a878c40
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FATAL_LORE,
    oracle_id = "aeb85786-16fc-45d8-9383-60054a878c40",
    scryfall_id = "24ba0b83-9671-4ee7-996d-57a3616b9c66",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Fatal Lore",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
