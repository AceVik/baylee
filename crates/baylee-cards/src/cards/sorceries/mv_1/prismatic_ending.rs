//! Prismatic Ending — {X}{W} — Sorcery
//! Oracle: Converge — Exile target nonland permanent if its mana value is less than or equal to the number of colors of mana spent to cast this spell.
//! Set: MH2 #25 — Modern Horizons 2 | Scryfall ID: 825969b9-3c70-4fca-8cab-696e9ca7cdb2 | Oracle ID: 2cb98ca9-d7bb-416b-a17e-ee5f8e4d78f2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PRISMATIC_ENDING,
    oracle_id = "2cb98ca9-d7bb-416b-a17e-ee5f8e4d78f2",
    scryfall_id = "825969b9-3c70-4fca-8cab-696e9ca7cdb2",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Prismatic Ending",
        mana_cost = mana!("{X}{W}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
