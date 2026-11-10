//! Diabolic Intent — {1}{B} — Sorcery
//! Oracle: As an additional cost to cast this spell, sacrifice a creature.
//! Oracle: Search your library for a card, put that card into your hand, then shuffle.
//! Set: BRO #89 — The Brothers' War | Scryfall ID: d72ab698-de67-4ca8-8e42-b05346bf52fa | Oracle ID: 038519b9-bca8-4b27-b5ac-2409595469d0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DIABOLIC_INTENT,
    oracle_id = "038519b9-bca8-4b27-b5ac-2409595469d0",
    scryfall_id = "d72ab698-de67-4ca8-8e42-b05346bf52fa",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Diabolic Intent",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
