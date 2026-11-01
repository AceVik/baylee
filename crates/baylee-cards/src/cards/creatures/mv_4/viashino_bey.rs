//! Viashino Bey — {2}{R}{R} — Creature — Lizard
//! Oracle: If this creature attacks, all creatures you control attack if able.
//! Set: ULG #93 — Urza's Legacy | Scryfall ID: 26cc07c6-60c7-4abe-8197-7544887ec64d | Oracle ID: d9c9b933-74f4-4898-bbb8-2da76b86ccda
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VIASHINO_BEY,
    oracle_id = "d9c9b933-74f4-4898-bbb8-2da76b86ccda",
    scryfall_id = "26cc07c6-60c7-4abe-8197-7544887ec64d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Viashino Bey",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LIZARD],
        power = Some(4),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
