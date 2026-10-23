//! Scorched Earth — {X}{R} — Sorcery
//! Oracle: As an additional cost to cast this spell, discard X land cards.
//! Oracle: Destroy X target lands.
//! Set: TMP #200 — Tempest | Scryfall ID: e6a97817-d1fd-4ba4-9ced-c2702b081523 | Oracle ID: fd1ce3e0-9c2d-48b6-9a14-4e71921cdfa4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SCORCHED_EARTH,
    oracle_id = "fd1ce3e0-9c2d-48b6-9a14-4e71921cdfa4",
    scryfall_id = "e6a97817-d1fd-4ba4-9ced-c2702b081523",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Scorched Earth",
        mana_cost = mana!("{X}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
