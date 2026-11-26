//! Flesh Reaver — {1}{B} — Creature — Phyrexian Horror
//! Oracle: Whenever this creature deals damage to a creature or opponent, this creature deals that much damage to you.
//! Set: DMR #86 — Dominaria Remastered | Scryfall ID: 5d56f46a-7327-4d98-b194-272333c8171e | Oracle ID: a143fdfe-f1aa-4eab-8c81-8c72a4dc1695
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FLESH_REAVER,
    oracle_id = "a143fdfe-f1aa-4eab-8c81-8c72a4dc1695",
    scryfall_id = "5d56f46a-7327-4d98-b194-272333c8171e",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Flesh Reaver",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::PHYREXIAN, subtypes::creature::HORROR],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
