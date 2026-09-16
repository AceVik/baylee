//! Lake-town Lookout — {W} — Creature — Human Scout
//! Oracle: When this creature dies, recruit. (Draw a card, then discard a card. If you discarded a nonland card, create a 1/1 white Human Soldier creature token.)
//! Set: HOB #18 — The Hobbit | Scryfall ID: 178c4cf6-6b11-40e4-9673-c560d6818a6b | Oracle ID: cf765efe-884c-48e2-9edb-9d45cf2756dd
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LAKE_TOWN_LOOKOUT,
    oracle_id = "cf765efe-884c-48e2-9edb-9d45cf2756dd",
    scryfall_id = "178c4cf6-6b11-40e4-9673-c560d6818a6b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Lake-town Lookout",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SCOUT],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
