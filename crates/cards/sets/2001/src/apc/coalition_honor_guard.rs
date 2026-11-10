//! Coalition Honor Guard — {3}{W} — Creature — Human Flagbearer
//! Oracle: While an opponent is choosing targets as part of casting a spell they control or activating an ability they control, that player must choose at least one Flagbearer on the battlefield if able.
//! Set: EMA #6 — Eternal Masters | Scryfall ID: 2c7c2b5c-634a-4d83-81bc-c6128e3ac339 | Oracle ID: c95cebf7-c28c-4453-b5e4-e895d621f290
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::COALITION_HONOR_GUARD,
    oracle_id = "c95cebf7-c28c-4453-b5e4-e895d621f290",
    scryfall_id = "2c7c2b5c-634a-4d83-81bc-c6128e3ac339",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Coalition Honor Guard",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::FLAGBEARER],
        power = Some(2),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
