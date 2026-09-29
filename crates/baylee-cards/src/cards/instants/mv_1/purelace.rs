//! Purelace — {W} — Instant
//! Oracle: Target spell or permanent becomes white. (Mana symbols on that permanent remain unchanged.)
//! Set: 4ED #43 — Fourth Edition | Scryfall ID: 5736fcc7-fa95-4ef4-b821-392ec00e03bf | Oracle ID: 3773001a-8868-49ec-a406-298cf72359c2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PURELACE,
    oracle_id = "3773001a-8868-49ec-a406-298cf72359c2",
    scryfall_id = "5736fcc7-fa95-4ef4-b821-392ec00e03bf",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Purelace",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
