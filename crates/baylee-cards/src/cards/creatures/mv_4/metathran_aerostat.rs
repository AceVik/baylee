//! Metathran Aerostat — {2}{U}{U} — Creature — Metathran
//! Oracle: Flying
//! Oracle: {X}{U}: You may put a creature card with mana value X from your hand onto the battlefield. If you do, return this creature to its owner's hand.
//! Set: INV #61 — Invasion | Scryfall ID: 59f34850-fb6f-4ac5-8309-4d53d770e28c | Oracle ID: 093daf01-d83b-4d49-a6b3-ec9a799b9582
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::METATHRAN_AEROSTAT,
    oracle_id = "093daf01-d83b-4d49-a6b3-ec9a799b9582",
    scryfall_id = "59f34850-fb6f-4ac5-8309-4d53d770e28c",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Metathran Aerostat",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::METATHRAN],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
