//! Suffocation — {1}{U} — Instant
//! Oracle: Cast this spell only if you were dealt damage this turn by a red instant or sorcery spell.
//! Oracle: Suffocation deals 4 damage to the controller of the last red instant or sorcery spell that dealt damage to you this turn.
//! Oracle: Draw a card at the beginning of the next turn's upkeep.
//! Set: ALL #38 — Alliances | Scryfall ID: d22104df-8147-45fd-897a-f99a815be062 | Oracle ID: 57aeb106-dc4f-4069-b596-f997263dc0cf
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SUFFOCATION,
    oracle_id = "57aeb106-dc4f-4069-b596-f997263dc0cf",
    scryfall_id = "d22104df-8147-45fd-897a-f99a815be062",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Suffocation",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
