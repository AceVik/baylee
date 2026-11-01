//! Winter's Chill — {X}{U} — Instant
//! Oracle: Cast this spell only during combat before blockers are declared.
//! Oracle: X can't be greater than the number of snow lands you control.
//! Oracle: Choose X target attacking creatures. For each of those creatures, its controller may pay {1} or {2}. If that player doesn't, destroy that creature at end of combat. If that player pays only {1}, prevent all combat damage that would be dealt to and dealt by that creature this combat.
//! Set: ICE #107 — Ice Age | Scryfall ID: a779aca7-ff2c-48d8-9484-6ad04b2c6bcb | Oracle ID: ae78d498-11c7-427f-9d13-3f1b7096f803
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WINTER_S_CHILL,
    oracle_id = "ae78d498-11c7-427f-9d13-3f1b7096f803",
    scryfall_id = "a779aca7-ff2c-48d8-9484-6ad04b2c6bcb",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Winter's Chill",
        mana_cost = mana!("{X}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
