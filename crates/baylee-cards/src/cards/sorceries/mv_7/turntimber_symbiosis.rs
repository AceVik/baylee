//! Turntimber Symbiosis // Turntimber, Serpentine Wood — {4}{G}{G}{G} — Sorcery // Land
//! Oracle: Look at the top seven cards of your library. You may put a creature card from among them onto the battlefield. If that card has mana value 3 or less, it enters with three additional +1/+1 counters on it. Put the rest on the bottom of your library in a random order.
//! Oracle: As this land enters, you may pay 3 life. If you don't, it enters tapped.
//! Oracle: {T}: Add {G}.
//! Set: ZNR #215 — Zendikar Rising | Scryfall ID: 61bd69ea-1e9e-46b0-b1a1-ed7fdbe3deb6 | Oracle ID: 403b59f3-7ade-4bc2-a3e6-de0c3c700f18
//! Face: Turntimber Symbiosis — {4}{G}{G}{G} — Sorcery
//! Face: Turntimber, Serpentine Wood —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = 1238,
    oracle_id = "403b59f3-7ade-4bc2-a3e6-de0c3c700f18",
    scryfall_id = "61bd69ea-1e9e-46b0-b1a1-ed7fdbe3deb6",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[
        face!(
            name = "Turntimber Symbiosis",
            mana_cost = mana!("{4}{G}{G}{G}"),
            types = TypeSet::SORCERY,
        ),
        face!(name = "Turntimber, Serpentine Wood", types = TypeSet::LAND,),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
