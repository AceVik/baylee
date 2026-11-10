//! Canopy Crawler — {3}{G} — Creature — Beast
//! Oracle: Amplify 1 (As this creature enters, put a +1/+1 counter on it for each Beast card you reveal in your hand.)
//! Oracle: {T}: Target creature gets +1/+1 until end of turn for each +1/+1 counter on this creature.
//! Set: LGN #122 — Legions | Scryfall ID: 0ccdc9d7-71b5-4304-8d19-a63952e17a6b | Oracle ID: 7fc60ae2-75fc-49e1-9d56-aa5a43844210
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CANOPY_CRAWLER,
    oracle_id = "7fc60ae2-75fc-49e1-9d56-aa5a43844210",
    scryfall_id = "0ccdc9d7-71b5-4304-8d19-a63952e17a6b",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Canopy Crawler",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
