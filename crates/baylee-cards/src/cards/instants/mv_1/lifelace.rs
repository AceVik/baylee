//! Lifelace — {G} — Instant
//! Oracle: Target spell or permanent becomes green. (Mana symbols on that permanent remain unchanged.)
//! Set: 4ED #258 — Fourth Edition | Scryfall ID: bf312acb-2fa6-440a-964b-424ad8abc331 | Oracle ID: eec1de80-4b3d-481d-a235-c299e0381830
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LIFELACE,
    oracle_id = "eec1de80-4b3d-481d-a235-c299e0381830",
    scryfall_id = "bf312acb-2fa6-440a-964b-424ad8abc331",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Lifelace",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
