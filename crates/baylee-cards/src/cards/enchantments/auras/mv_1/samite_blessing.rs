//! Samite Blessing — {W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has "{T}: The next time a source of your choice would deal damage to target creature this turn, prevent that damage."
//! Set: STH #13 — Stronghold | Scryfall ID: 3b21aa2e-df36-4101-89b6-515858f2ab88 | Oracle ID: a524ae69-2594-467d-af0a-27b9984d4300
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SAMITE_BLESSING,
    oracle_id = "a524ae69-2594-467d-af0a-27b9984d4300",
    scryfall_id = "3b21aa2e-df36-4101-89b6-515858f2ab88",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Samite Blessing",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
