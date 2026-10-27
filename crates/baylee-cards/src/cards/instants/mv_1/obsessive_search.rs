//! Obsessive Search — {U} — Instant
//! Oracle: Draw a card.
//! Oracle: Madness {U} (If you discard this card, discard it into exile. When you do, cast it for its madness cost or put it into your graveyard.)
//! Set: DMR #61 — Dominaria Remastered | Scryfall ID: 1d97436e-329a-49a4-a7c9-671ec9ba90cc | Oracle ID: 116ee88e-9e13-4f21-bc6a-efa994b5bf75
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OBSESSIVE_SEARCH,
    oracle_id = "116ee88e-9e13-4f21-bc6a-efa994b5bf75",
    scryfall_id = "1d97436e-329a-49a4-a7c9-671ec9ba90cc",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Obsessive Search",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
