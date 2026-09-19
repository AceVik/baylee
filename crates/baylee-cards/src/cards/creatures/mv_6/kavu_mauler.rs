//! Kavu Mauler — {4}{G}{G} — Creature — Kavu
//! Oracle: Trample
//! Oracle: Whenever this creature attacks, it gets +1/+1 until end of turn for each other attacking Kavu.
//! Set: APC #80 — Apocalypse | Scryfall ID: 79adc3af-5fa3-4cb6-9bbc-52ede0c69263 | Oracle ID: 6ffbcaba-5437-4fb6-a2d6-e94b1e6dc1d2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KAVU_MAULER,
    oracle_id = "6ffbcaba-5437-4fb6-a2d6-e94b1e6dc1d2",
    scryfall_id = "79adc3af-5fa3-4cb6-9bbc-52ede0c69263",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Kavu Mauler",
        mana_cost = mana!("{4}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::KAVU],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
