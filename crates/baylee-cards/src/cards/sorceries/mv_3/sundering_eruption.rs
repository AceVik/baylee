//! Sundering Eruption // Volcanic Fissure — {2}{R} — Sorcery // Land
//! Oracle: Destroy target land. Its controller may search their library for a basic land card, put it onto the battlefield tapped, then shuffle. Creatures without flying can't block this turn.
//! Oracle: As this land enters, you may pay 3 life. If you don't, it enters tapped.
//! Oracle: {T}: Add {R}.
//! Set: MH3 #248 — Modern Horizons 3 | Scryfall ID: 50686ac7-346c-43d1-bdaa-28d46a12ad93 | Oracle ID: c95309e9-5c2f-4518-b2fd-825d3d0a4ae0
//! Face: Sundering Eruption — {2}{R} — Sorcery
//! Face: Volcanic Fissure —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = 28386,
    oracle_id = "c95309e9-5c2f-4518-b2fd-825d3d0a4ae0",
    scryfall_id = "50686ac7-346c-43d1-bdaa-28d46a12ad93",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[
        face!(
            name = "Sundering Eruption",
            mana_cost = mana!("{2}{R}"),
            types = TypeSet::SORCERY,
        ),
        face!(name = "Volcanic Fissure", types = TypeSet::LAND,),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
