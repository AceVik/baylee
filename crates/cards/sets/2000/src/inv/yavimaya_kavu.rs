//! Yavimaya Kavu — {2}{R}{G} — Creature — Kavu
//! Oracle: Yavimaya Kavu's power is equal to the number of red creatures on the battlefield.
//! Oracle: Yavimaya Kavu's toughness is equal to the number of green creatures on the battlefield.
//! Set: INV #291 — Invasion | Scryfall ID: 1872f104-7cf1-41e3-b1b4-ca75c678e08b | Oracle ID: 285c9f49-4aeb-468c-bb41-f5ccb5257e2b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::YAVIMAYA_KAVU,
    oracle_id = "285c9f49-4aeb-468c-bb41-f5ccb5257e2b",
    scryfall_id = "1872f104-7cf1-41e3-b1b4-ca75c678e08b",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    faces = &[face!(
        name = "Yavimaya Kavu",
        mana_cost = mana!("{2}{R}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::KAVU],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
