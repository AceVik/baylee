//! Coral Reef — {U}{U} — Enchantment
//! Oracle: This enchantment enters with four polyp counters on it.
//! Oracle: Sacrifice an Island: Put two polyp counters on this enchantment.
//! Oracle: {U}, Tap an untapped blue creature you control, Remove a polyp counter from this enchantment: Put a +0/+1 counter on target creature.
//! Set: HML #24 — Homelands | Scryfall ID: 42fe2280-a996-4072-b5bf-f4fd56607a51 | Oracle ID: 37a40a53-2668-4521-a786-c5747eedeeb2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CORAL_REEF,
    oracle_id = "37a40a53-2668-4521-a786-c5747eedeeb2",
    scryfall_id = "42fe2280-a996-4072-b5bf-f4fd56607a51",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Coral Reef",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
