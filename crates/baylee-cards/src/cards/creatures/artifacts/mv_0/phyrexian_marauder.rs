//! Phyrexian Marauder — {X} — Artifact Creature — Phyrexian Construct
//! Oracle: This creature enters with X +1/+1 counters on it.
//! Oracle: This creature can't block.
//! Oracle: This creature can't attack unless you pay {1} for each +1/+1 counter on it.
//! Set: VIS #151 — Visions | Scryfall ID: 29a75dc8-1c24-4063-8944-d7e71b4a5755 | Oracle ID: 405674c5-607a-43d3-8b46-8741d2339f80
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PHYREXIAN_MARAUDER,
    oracle_id = "405674c5-607a-43d3-8b46-8741d2339f80",
    scryfall_id = "29a75dc8-1c24-4063-8944-d7e71b4a5755",
    faces = &[face!(
        name = "Phyrexian Marauder",
        mana_cost = mana!("{X}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::PHYREXIAN, subtypes::creature::CONSTRUCT],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
