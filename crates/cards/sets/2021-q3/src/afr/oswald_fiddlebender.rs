//! Oswald Fiddlebender — {1}{W} — Legendary Creature — Gnome Artificer
//! Oracle: Magical Tinkering — {W}, {T}, Sacrifice an artifact: Search your library for an artifact card with mana value equal to 1 plus the sacrificed artifact's mana value, put it onto the battlefield, then shuffle. Activate only as a sorcery.
//! Set: AFR #28 — Adventures in the Forgotten Realms | Scryfall ID: bba1650f-eddf-49a9-820e-489cb8d5b6fa | Oracle ID: dbc9ea19-cf68-41d3-88a8-b5ab8df75c5a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::OSWALD_FIDDLEBENDER,
    oracle_id = "dbc9ea19-cf68-41d3-88a8-b5ab8df75c5a",
    scryfall_id = "bba1650f-eddf-49a9-820e-489cb8d5b6fa",
    color_identity = ColorSet::from_slice(&[Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Oswald Fiddlebender",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::GNOME, subtypes::creature::ARTIFICER],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
