//! Fork — {R}{R} — Instant
//! Oracle: Copy target instant or sorcery spell, except that the copy is red. You may choose new targets for the copy.
//! Set: ME4 #116 — Masters Edition IV | Scryfall ID: e4ff994a-bddd-486d-9a7b-a8959b4cf1dd | Oracle ID: 50c53ae0-51ba-4046-ac74-87c65e688032
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FORK,
    oracle_id = "50c53ae0-51ba-4046-ac74-87c65e688032",
    scryfall_id = "e4ff994a-bddd-486d-9a7b-a8959b4cf1dd",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Fork",
        mana_cost = mana!("{R}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
