//! Liege of the Hollows — {2}{G}{G} — Creature — Spirit
//! Oracle: When this creature dies, each player may pay any amount of mana. Then each player creates a number of 1/1 green Squirrel creature tokens equal to the amount of mana they paid this way.
//! Set: WTH #131 — Weatherlight | Scryfall ID: dff4512b-8244-4e38-bffb-0062a97d9531 | Oracle ID: bef7441e-5a44-46bc-8141-8e02ecd06fce
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LIEGE_OF_THE_HOLLOWS,
    oracle_id = "bef7441e-5a44-46bc-8141-8e02ecd06fce",
    scryfall_id = "dff4512b-8244-4e38-bffb-0062a97d9531",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Liege of the Hollows",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(3),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
