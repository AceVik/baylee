//! Gorilla Titan — {3}{G}{G} — Creature — Ape
//! Oracle: Trample
//! Oracle: This creature gets +4/+4 as long as there are no cards in your graveyard.
//! Set: ODY #241 — Odyssey | Scryfall ID: 435d9562-8f2b-43fe-ba21-8f5896378280 | Oracle ID: 9ce80c55-a995-4226-8506-901f6cf06efc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GORILLA_TITAN,
    oracle_id = "9ce80c55-a995-4226-8506-901f6cf06efc",
    scryfall_id = "435d9562-8f2b-43fe-ba21-8f5896378280",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Gorilla Titan",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::APE],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
