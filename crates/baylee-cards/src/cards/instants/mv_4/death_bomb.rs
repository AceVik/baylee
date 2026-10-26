//! Death Bomb — {3}{B} — Instant
//! Oracle: As an additional cost to cast this spell, sacrifice a creature.
//! Oracle: Destroy target nonblack creature. It can't be regenerated. Its controller loses 2 life.
//! Set: PLS #41 — Planeshift | Scryfall ID: f8a84715-c5dc-4a19-af6a-796c6ee912c2 | Oracle ID: e323d468-def5-4090-af5b-19481289168f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DEATH_BOMB,
    oracle_id = "e323d468-def5-4090-af5b-19481289168f",
    scryfall_id = "f8a84715-c5dc-4a19-af6a-796c6ee912c2",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Death Bomb",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
