//! Firebolt — {R} — Sorcery
//! Oracle: Firebolt deals 2 damage to any target.
//! Oracle: Flashback {4}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: MH1 #122 — Modern Horizons | Scryfall ID: 882b8f98-ee51-4c94-a3eb-c79eb2b50d78 | Oracle ID: 7aa31280-12c3-479d-a6dd-f1c669043083
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FIREBOLT,
    oracle_id = "7aa31280-12c3-479d-a6dd-f1c669043083",
    scryfall_id = "882b8f98-ee51-4c94-a3eb-c79eb2b50d78",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Firebolt",
        mana_cost = mana!("{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
