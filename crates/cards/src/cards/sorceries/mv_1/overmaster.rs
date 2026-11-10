//! Overmaster — {R} — Sorcery
//! Oracle: The next instant or sorcery spell you cast this turn can't be countered.
//! Oracle: Draw a card.
//! Set: DMR #132 — Dominaria Remastered | Scryfall ID: 28e812a8-66cf-4005-a647-780d49aa74a4 | Oracle ID: acc842a4-26e8-435a-b717-c1043b581ff8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OVERMASTER,
    oracle_id = "acc842a4-26e8-435a-b717-c1043b581ff8",
    scryfall_id = "28e812a8-66cf-4005-a647-780d49aa74a4",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Overmaster",
        mana_cost = mana!("{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
