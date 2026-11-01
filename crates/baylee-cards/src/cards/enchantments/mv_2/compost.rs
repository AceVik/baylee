//! Compost — {1}{G} — Enchantment
//! Oracle: Whenever a black card is put into an opponent's graveyard from anywhere, you may draw a card.
//! Set: 7ED #235 — Seventh Edition | Scryfall ID: 91fc8eca-7549-45b5-b3db-89a58d7d2a4a | Oracle ID: 4681bcc2-8a46-44cd-8b04-6ab140904dea
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::COMPOST,
    oracle_id = "4681bcc2-8a46-44cd-8b04-6ab140904dea",
    scryfall_id = "91fc8eca-7549-45b5-b3db-89a58d7d2a4a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Compost",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
