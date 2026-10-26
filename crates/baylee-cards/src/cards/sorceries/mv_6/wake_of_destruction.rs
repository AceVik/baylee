//! Wake of Destruction — {3}{R}{R}{R} — Sorcery
//! Oracle: Destroy target land and all other lands with the same name as that land.
//! Set: UDS #99 — Urza's Destiny | Scryfall ID: 0c070f12-0342-48d5-ab0e-4fc4701c3669 | Oracle ID: 562f7f99-ee67-4a14-b5e2-1a4ae710726d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WAKE_OF_DESTRUCTION,
    oracle_id = "562f7f99-ee67-4a14-b5e2-1a4ae710726d",
    scryfall_id = "0c070f12-0342-48d5-ab0e-4fc4701c3669",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Wake of Destruction",
        mana_cost = mana!("{3}{R}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
