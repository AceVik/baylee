//! Truce — {2}{W} — Instant
//! Oracle: Each player may draw up to two cards. For each card less than two a player draws this way, that player gains 2 life.
//! Set: 5ED #65 — Fifth Edition | Scryfall ID: de4f53fc-69d7-49fb-885b-4cc02bf5facc | Oracle ID: fe4aa72b-f176-461a-aca0-1519721609b2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TRUCE,
    oracle_id = "fe4aa72b-f176-461a-aca0-1519721609b2",
    scryfall_id = "de4f53fc-69d7-49fb-885b-4cc02bf5facc",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Truce",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
