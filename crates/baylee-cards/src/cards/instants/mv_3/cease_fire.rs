//! Cease-Fire — {2}{W} — Instant
//! Oracle: Target player can't cast creature spells this turn.
//! Oracle: Draw a card.
//! Set: ODY #14 — Odyssey | Scryfall ID: 1646c998-aee5-497f-bd75-24ced1dabef9 | Oracle ID: 9e48eb8e-ec62-4d73-ac80-31e6fe74ec31
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CEASE_FIRE,
    oracle_id = "9e48eb8e-ec62-4d73-ac80-31e6fe74ec31",
    scryfall_id = "1646c998-aee5-497f-bd75-24ced1dabef9",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Cease-Fire",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
