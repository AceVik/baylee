//! Thought Lash — {2}{U}{U} — Enchantment
//! Oracle: Cumulative upkeep—Exile the top card of your library. (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: When a player doesn't pay this enchantment's cumulative upkeep, that player exiles all cards from their library.
//! Oracle: Exile the top card of your library: Prevent the next 1 damage that would be dealt to you this turn.
//! Set: ME2 #70 — Masters Edition II | Scryfall ID: 547b1405-3f4e-4c47-8600-3e92197b1dd7 | Oracle ID: 81fe7c5e-503e-488c-b06c-1615ed38e5a3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THOUGHT_LASH,
    oracle_id = "81fe7c5e-503e-488c-b06c-1615ed38e5a3",
    scryfall_id = "547b1405-3f4e-4c47-8600-3e92197b1dd7",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Thought Lash",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
