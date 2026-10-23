//! Cannibalize — {1}{B} — Sorcery
//! Oracle: Choose two target creatures controlled by the same player. Exile one of those creatures and put two +1/+1 counters on the other.
//! Set: TPR #83 — Tempest Remastered | Scryfall ID: 9cdf1311-1660-4ad0-a383-f3ff508ff574 | Oracle ID: bf044267-92b9-4983-964f-2bc4ecf76bce
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CANNIBALIZE,
    oracle_id = "bf044267-92b9-4983-964f-2bc4ecf76bce",
    scryfall_id = "9cdf1311-1660-4ad0-a383-f3ff508ff574",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Cannibalize",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
