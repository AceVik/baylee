//! Bellowing Fiend — {4}{B} — Creature — Spirit
//! Oracle: Flying
//! Oracle: Whenever this creature deals damage to a creature, this creature deals 3 damage to that creature's controller and 3 damage to you.
//! Set: 7ED #119 — Seventh Edition | Scryfall ID: 2b0962d7-d797-4f07-bd73-9cd7a11ffad8 | Oracle ID: 37331d0e-edda-4791-8bcc-fb457f984877
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BELLOWING_FIEND,
    oracle_id = "37331d0e-edda-4791-8bcc-fb457f984877",
    scryfall_id = "2b0962d7-d797-4f07-bd73-9cd7a11ffad8",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Bellowing Fiend",
        mana_cost = mana!("{4}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
