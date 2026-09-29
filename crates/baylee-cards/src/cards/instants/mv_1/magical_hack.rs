//! Magical Hack — {U} — Instant
//! Oracle: Change the text of target spell or permanent by replacing all instances of one basic land type with another. (For example, you may change "swampwalk" to "plainswalk." This effect lasts indefinitely.)
//! Set: 5ED #101 — Fifth Edition | Scryfall ID: c2349d53-d9f0-450f-be96-463791ee7aab | Oracle ID: cba229fa-9035-405b-b091-3798898a37ee
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MAGICAL_HACK,
    oracle_id = "cba229fa-9035-405b-b091-3798898a37ee",
    scryfall_id = "c2349d53-d9f0-450f-be96-463791ee7aab",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Magical Hack",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
