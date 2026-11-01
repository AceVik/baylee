//! Corrosion — {1}{B}{R} — Enchantment
//! Oracle: Cumulative upkeep {1} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: At the beginning of your upkeep, put a rust counter on each artifact target opponent controls. Then destroy each artifact with mana value less than or equal to the number of rust counters on it. Artifacts destroyed this way can't be regenerated.
//! Oracle: When this enchantment leaves the battlefield, remove all rust counters from all permanents.
//! Set: VIS #128 — Visions | Scryfall ID: 176122b2-f60f-4150-8c0c-757c8f8914d2 | Oracle ID: 3d3fd8a2-78d4-4f52-8fcd-0c6fa93f0043
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CORROSION,
    oracle_id = "3d3fd8a2-78d4-4f52-8fcd-0c6fa93f0043",
    scryfall_id = "176122b2-f60f-4150-8c0c-757c8f8914d2",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces = &[face!(
        name = "Corrosion",
        mana_cost = mana!("{1}{B}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
