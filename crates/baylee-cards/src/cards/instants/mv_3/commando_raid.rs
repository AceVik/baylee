//! Commando Raid — {2}{R} — Instant
//! Oracle: Until end of turn, target creature you control gains "When this creature deals combat damage to a player, you may have it deal damage equal to its power to target creature that player controls."
//! Set: ONS #195 — Onslaught | Scryfall ID: bb237330-ac2e-411d-836c-6628f96f3262 | Oracle ID: 3bd57758-00a6-4d49-b1fc-b14f8203da0b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::COMMANDO_RAID,
    oracle_id = "3bd57758-00a6-4d49-b1fc-b14f8203da0b",
    scryfall_id = "bb237330-ac2e-411d-836c-6628f96f3262",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Commando Raid",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
