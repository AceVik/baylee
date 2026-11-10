//! Soul Echo — {X}{W}{W} — Enchantment
//! Oracle: This enchantment enters with X echo counters on it.
//! Oracle: You don't lose the game for having 0 or less life.
//! Oracle: At the beginning of your upkeep, sacrifice this enchantment if there are no echo counters on it. Otherwise, target opponent may choose that for each 1 damage that would be dealt to you until your next upkeep, you remove an echo counter from this enchantment instead.
//! Set: MIR #40 — Mirage | Scryfall ID: cb6f9427-ef5c-49fd-81e1-ddf130d69da0 | Oracle ID: 8bc5546e-554f-430d-80b1-99578b5ea188
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SOUL_ECHO,
    oracle_id = "8bc5546e-554f-430d-80b1-99578b5ea188",
    scryfall_id = "cb6f9427-ef5c-49fd-81e1-ddf130d69da0",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Soul Echo",
        mana_cost = mana!("{X}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
