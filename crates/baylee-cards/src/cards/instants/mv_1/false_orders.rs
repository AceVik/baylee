//! False Orders — {R} — Instant
//! Oracle: Cast this spell only during the declare blockers step.
//! Oracle: Remove target creature defending player controls from combat. Creatures it was blocking that had become blocked by only that creature this combat become unblocked. You may have it block an attacking creature of your choice.
//! Set: 2ED #148 — Unlimited Edition | Scryfall ID: a59c24d9-804b-45d0-b60c-cfc7a6af7ef5 | Oracle ID: 38c5c952-8153-4d98-89b5-a75260383345
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FALSE_ORDERS,
    oracle_id = "38c5c952-8153-4d98-89b5-a75260383345",
    scryfall_id = "a59c24d9-804b-45d0-b60c-cfc7a6af7ef5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "False Orders",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
