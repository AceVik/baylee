//! Standard Bearer — {1}{W} — Creature — Human Flagbearer
//! Oracle: While an opponent is choosing targets as part of casting a spell they control or activating an ability they control, that player must choose at least one Flagbearer on the battlefield if able.
//! Set: APC #18 — Apocalypse | Scryfall ID: e0f8e16a-55f0-4147-a01a-dba7938f31c4 | Oracle ID: 1d6e9716-8c3c-443d-afd8-68d0d6c469a5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::STANDARD_BEARER,
    oracle_id = "1d6e9716-8c3c-443d-afd8-68d0d6c469a5",
    scryfall_id = "e0f8e16a-55f0-4147-a01a-dba7938f31c4",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Standard Bearer",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::FLAGBEARER],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
