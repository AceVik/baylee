//! Stench of Evil — {2}{B}{B} — Sorcery
//! Oracle: Destroy all Plains. For each land destroyed this way, Stench of Evil deals 1 damage to that land's controller unless they pay {2}.
//! Set: ICE #165 — Ice Age | Scryfall ID: 4c7065a2-f819-4cbe-b453-a55e904f0461 | Oracle ID: 14062abe-14df-4dbe-ad7b-530e5f6c4988
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STENCH_OF_EVIL,
    oracle_id = "14062abe-14df-4dbe-ad7b-530e5f6c4988",
    scryfall_id = "4c7065a2-f819-4cbe-b453-a55e904f0461",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Stench of Evil",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
