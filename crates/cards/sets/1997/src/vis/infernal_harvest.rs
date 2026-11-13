//! Infernal Harvest — {1}{B} — Sorcery
//! Oracle: As an additional cost to cast this spell, return X Swamps you control to their owner's hand.
//! Oracle: Infernal Harvest deals X damage divided as you choose among any number of target creatures.
//! Set: VIS #62 — Visions | Scryfall ID: ccf85ac9-f5d8-4a36-aa6c-3a31427a0348 | Oracle ID: b8c8cde9-34db-4185-86f5-4289969880c5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::INFERNAL_HARVEST,
    oracle_id = "b8c8cde9-34db-4185-86f5-4289969880c5",
    scryfall_id = "ccf85ac9-f5d8-4a36-aa6c-3a31427a0348",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Infernal Harvest",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
