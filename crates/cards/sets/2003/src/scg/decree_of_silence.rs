//! Decree of Silence — {6}{U}{U} — Enchantment
//! Oracle: Whenever an opponent casts a spell, counter that spell and put a depletion counter on this enchantment. If there are three or more depletion counters on this enchantment, sacrifice it.
//! Oracle: Cycling {4}{U}{U} ({4}{U}{U}, Discard this card: Draw a card.)
//! Oracle: When you cycle this card, you may counter target spell.
//! Set: SCG #32 — Scourge | Scryfall ID: f2fc46e2-5e19-4999-a4cd-1e84697066c1 | Oracle ID: 920684d0-aef3-4408-a66b-a0c9394c58f4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DECREE_OF_SILENCE,
    oracle_id = "920684d0-aef3-4408-a66b-a0c9394c58f4",
    scryfall_id = "f2fc46e2-5e19-4999-a4cd-1e84697066c1",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Decree of Silence",
        mana_cost = mana!("{6}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
