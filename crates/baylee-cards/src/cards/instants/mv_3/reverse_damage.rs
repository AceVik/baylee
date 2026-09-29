//! Reverse Damage — {1}{W}{W} — Instant
//! Oracle: The next time a source of your choice would deal damage to you this turn, prevent that damage. You gain life equal to the damage prevented this way.
//! Set: 9ED #35 — Ninth Edition | Scryfall ID: 5f938c52-c7f6-41a4-b480-632b43b60b67 | Oracle ID: eaaf7c30-f463-4115-a40e-7dc717063413
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REVERSE_DAMAGE,
    oracle_id = "eaaf7c30-f463-4115-a40e-7dc717063413",
    scryfall_id = "5f938c52-c7f6-41a4-b480-632b43b60b67",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Reverse Damage",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
