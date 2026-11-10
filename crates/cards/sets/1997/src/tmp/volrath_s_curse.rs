//! Volrath's Curse — {1}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature can't attack or block, and its activated abilities can't be activated. That creature's controller may sacrifice a permanent of their choice for that player to ignore this effect until end of turn.
//! Oracle: {1}{U}: Return this Aura to its owner's hand.
//! Set: TPR #77 — Tempest Remastered | Scryfall ID: c80be3e6-37da-43bd-b959-9dbfead3baa1 | Oracle ID: f4ff3efb-c9ed-4451-a7f2-5ff52ffb6cab
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VOLRATH_S_CURSE,
    oracle_id = "f4ff3efb-c9ed-4451-a7f2-5ff52ffb6cab",
    scryfall_id = "c80be3e6-37da-43bd-b959-9dbfead3baa1",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Volrath's Curse",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
