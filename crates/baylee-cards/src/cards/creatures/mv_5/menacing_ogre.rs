//! Menacing Ogre — {3}{R}{R} — Creature — Ogre
//! Oracle: Trample, haste
//! Oracle: When this creature enters, each player secretly chooses a number. Then those numbers are revealed. Each player with the highest number loses that much life. If you are one of those players, put two +1/+1 counters on this creature.
//! Set: HOP #59 — Planechase | Scryfall ID: 8de4b5cc-748c-45d1-852b-0f522b8838ba | Oracle ID: 8efcd591-c576-4d25-9670-578c5e3263a1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MENACING_OGRE,
    oracle_id = "8efcd591-c576-4d25-9670-578c5e3263a1",
    scryfall_id = "8de4b5cc-748c-45d1-852b-0f522b8838ba",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Menacing Ogre",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::OGRE],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
