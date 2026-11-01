//! Corpse Dance — {2}{B} — Instant
//! Oracle: Buyback {2} (You may pay an additional {2} as you cast this spell. If you do, put this card into your hand as it resolves.)
//! Oracle: Return the top creature card of your graveyard to the battlefield. That creature gains haste until end of turn. Exile it at the beginning of the next end step.
//! Set: TPR #89 — Tempest Remastered | Scryfall ID: aaed9b00-f6a1-4023-b9b9-22d904fe5271 | Oracle ID: 59b02ca7-e7bd-4820-ac32-87ea1a8c64c0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CORPSE_DANCE,
    oracle_id = "59b02ca7-e7bd-4820-ac32-87ea1a8c64c0",
    scryfall_id = "aaed9b00-f6a1-4023-b9b9-22d904fe5271",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Corpse Dance",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
