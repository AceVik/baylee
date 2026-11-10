//! Mogg Conscripts — {R} — Creature — Goblin
//! Oracle: This creature can't attack unless you've cast a creature spell this turn.
//! Set: TPR #143 — Tempest Remastered | Scryfall ID: c67c26d7-f752-40a5-b79e-1f4c531cbef5 | Oracle ID: b015686d-07e1-46c7-8e51-2bec646860d6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MOGG_CONSCRIPTS,
    oracle_id = "b015686d-07e1-46c7-8e51-2bec646860d6",
    scryfall_id = "c67c26d7-f752-40a5-b79e-1f4c531cbef5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Mogg Conscripts",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
