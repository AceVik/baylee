//! Ivory Gargoyle — {4}{W} — Creature — Gargoyle
//! Oracle: Flying
//! Oracle: When this creature dies, return it to the battlefield under its owner's control at the beginning of the next end step and you skip your next draw step.
//! Oracle: {4}{W}: Exile this creature.
//! Set: ME2 #19 — Masters Edition II | Scryfall ID: 07d79058-2a78-4237-83b0-4715053e8267 | Oracle ID: e8cc9f83-4e4c-4820-9a7a-f82f728e9767
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::IVORY_GARGOYLE,
    oracle_id = "e8cc9f83-4e4c-4820-9a7a-f82f728e9767",
    scryfall_id = "07d79058-2a78-4237-83b0-4715053e8267",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Ivory Gargoyle",
        mana_cost = mana!("{4}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GARGOYLE],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
