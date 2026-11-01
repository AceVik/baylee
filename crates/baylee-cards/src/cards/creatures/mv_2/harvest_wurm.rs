//! Harvest Wurm — {1}{G} — Creature — Wurm
//! Oracle: When this creature enters, sacrifice it unless you return a basic land card from your graveyard to your hand.
//! Set: WTH #130 — Weatherlight | Scryfall ID: 9d21139d-edfc-4140-aa43-d4165331d7f3 | Oracle ID: cd539212-ef7b-4f78-8d62-c42289aedc51
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HARVEST_WURM,
    oracle_id = "cd539212-ef7b-4f78-8d62-c42289aedc51",
    scryfall_id = "9d21139d-edfc-4140-aa43-d4165331d7f3",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Harvest Wurm",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WURM],
        power = Some(3),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
