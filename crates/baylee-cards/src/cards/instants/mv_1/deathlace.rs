//! Deathlace — {B} — Instant
//! Oracle: Target spell or permanent becomes black. (Mana symbols on that permanent remain unchanged.)
//! Set: 4ED #131 — Fourth Edition | Scryfall ID: 237e37fb-383d-432c-8ac3-1332096567db | Oracle ID: fb80aaba-352a-4b58-8db2-1e02d542819c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DEATHLACE,
    oracle_id = "fb80aaba-352a-4b58-8db2-1e02d542819c",
    scryfall_id = "237e37fb-383d-432c-8ac3-1332096567db",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Deathlace",
        mana_cost = mana!("{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
