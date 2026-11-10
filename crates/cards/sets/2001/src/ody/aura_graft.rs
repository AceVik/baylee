//! Aura Graft — {1}{U} — Instant
//! Oracle: Gain control of target Aura that's attached to a permanent. Attach it to another permanent it can enchant.
//! Set: 10E #67 — Tenth Edition | Scryfall ID: 55552a2b-1861-4235-a60d-ccabb4839d54 | Oracle ID: 00317e4f-0228-469a-9789-fb6c39976860
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AURA_GRAFT,
    oracle_id = "00317e4f-0228-469a-9789-fb6c39976860",
    scryfall_id = "55552a2b-1861-4235-a60d-ccabb4839d54",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Aura Graft",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
