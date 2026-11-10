//! Orcish Mine — {1}{R}{R} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: This Aura enters with three ore counters on it.
//! Oracle: At the beginning of your upkeep and whenever enchanted land becomes tapped, remove an ore counter from this Aura.
//! Oracle: When the last ore counter is removed from this Aura, destroy enchanted land and this Aura deals 2 damage to that land's controller.
//! Set: HML #78 — Homelands | Scryfall ID: 3a630875-b43d-4591-992c-117e1212fa34 | Oracle ID: a6bfa03e-6317-4550-960b-fbd2a30e521f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ORCISH_MINE,
    oracle_id = "a6bfa03e-6317-4550-960b-fbd2a30e521f",
    scryfall_id = "3a630875-b43d-4591-992c-117e1212fa34",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Orcish Mine",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
