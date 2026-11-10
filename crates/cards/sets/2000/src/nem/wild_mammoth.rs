//! Wild Mammoth — {2}{G} — Creature — Elephant
//! Oracle: At the beginning of your upkeep, if a player controls more creatures than each other player, the player who controls the most creatures gains control of this creature.
//! Set: NEM #124 — Nemesis | Scryfall ID: 4d927fdb-11c0-42b6-95d4-7af051e45213 | Oracle ID: 29ef65bf-1066-44a5-a2eb-e3670b076647
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WILD_MAMMOTH,
    oracle_id = "29ef65bf-1066-44a5-a2eb-e3670b076647",
    scryfall_id = "4d927fdb-11c0-42b6-95d4-7af051e45213",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Wild Mammoth",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEPHANT],
        power = Some(3),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
