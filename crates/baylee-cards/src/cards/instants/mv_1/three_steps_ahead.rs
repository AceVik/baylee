//! Three Steps Ahead — {U} — Instant
//! Oracle: Spree (Choose one or more additional costs.)
//! Oracle: + {1}{U} — Counter target spell.
//! Oracle: + {3} — Create a token that's a copy of target artifact or creature you control.
//! Oracle: + {2} — Draw two cards, then discard a card.
//! Set: OTJ #75 — Outlaws of Thunder Junction | Scryfall ID: 8fffd839-2337-4a14-9312-cee085a17f4b | Oracle ID: 282dfeaa-6243-4f92-838a-5cb54fa85184
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THREE_STEPS_AHEAD,
    oracle_id = "282dfeaa-6243-4f92-838a-5cb54fa85184",
    scryfall_id = "8fffd839-2337-4a14-9312-cee085a17f4b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Three Steps Ahead",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
