//! Rooting Kavu — {2}{G}{G} — Creature — Kavu
//! Oracle: When this creature dies, you may exile it. If you do, shuffle all creature cards from your graveyard into your library.
//! Set: INV #207 — Invasion | Scryfall ID: 12c25a4c-d93a-402b-999f-0b9919123cc5 | Oracle ID: e7438616-cc42-4a6d-a7e3-5856cb0782fa
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ROOTING_KAVU,
    oracle_id = "e7438616-cc42-4a6d-a7e3-5856cb0782fa",
    scryfall_id = "12c25a4c-d93a-402b-999f-0b9919123cc5",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Rooting Kavu",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::KAVU],
        power = Some(4),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
