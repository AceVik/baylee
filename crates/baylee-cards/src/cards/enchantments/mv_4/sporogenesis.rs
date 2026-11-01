//! Sporogenesis — {3}{G} — Enchantment
//! Oracle: At the beginning of your upkeep, you may put a fungus counter on target nontoken creature.
//! Oracle: Whenever a creature with a fungus counter on it dies, create a 1/1 green Saproling creature token for each fungus counter on that creature.
//! Oracle: When this enchantment leaves the battlefield, remove all fungus counters from all creatures.
//! Set: USG #273 — Urza's Saga | Scryfall ID: 839d969b-b1f7-4978-b00c-db0766161f63 | Oracle ID: 33005c63-fe9f-4051-b781-eacf1ca5eec6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPOROGENESIS,
    oracle_id = "33005c63-fe9f-4051-b781-eacf1ca5eec6",
    scryfall_id = "839d969b-b1f7-4978-b00c-db0766161f63",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Sporogenesis",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
