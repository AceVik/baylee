//! Lat-Nam's Legacy — {1}{U} — Instant
//! Oracle: Shuffle a card from your hand into your library. If you do, draw two cards at the beginning of the next turn's upkeep.
//! Set: ME2 #52 — Masters Edition II | Scryfall ID: ce13ff7b-a03c-488f-a140-2c03b64fe218 | Oracle ID: f885ee54-3b26-4d61-9783-3930b0e625aa
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LAT_NAM_S_LEGACY,
    oracle_id = "f885ee54-3b26-4d61-9783-3930b0e625aa",
    scryfall_id = "ce13ff7b-a03c-488f-a140-2c03b64fe218",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Lat-Nam's Legacy",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
