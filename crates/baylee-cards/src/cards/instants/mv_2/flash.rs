//! Flash — {1}{U} — Instant
//! Oracle: You may put a creature card from your hand onto the battlefield. If you do, sacrifice it unless you pay its mana cost reduced by {2}.
//! Set: A25 #57 — Masters 25 | Scryfall ID: d31459c2-9656-4e9a-bb72-71a910e8570b | Oracle ID: 3e057471-e415-4105-9064-8236a0a5047d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FLASH,
    oracle_id = "3e057471-e415-4105-9064-8236a0a5047d",
    scryfall_id = "d31459c2-9656-4e9a-bb72-71a910e8570b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Flash",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
