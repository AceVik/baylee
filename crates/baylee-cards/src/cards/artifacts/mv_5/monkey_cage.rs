//! Monkey Cage — {5} — Artifact
//! Oracle: When a creature enters, sacrifice this artifact and create X 2/2 green Monkey creature tokens, where X is that creature's mana value.
//! Set: MMQ #307 — Mercadian Masques | Scryfall ID: 07f6be53-7a20-4e6b-a6ce-11cba06af8cb | Oracle ID: 33a11c01-89a6-446b-84da-e48fd1f27446
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MONKEY_CAGE,
    oracle_id = "33a11c01-89a6-446b-84da-e48fd1f27446",
    scryfall_id = "07f6be53-7a20-4e6b-a6ce-11cba06af8cb",
    faces = &[face!(
        name = "Monkey Cage",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
