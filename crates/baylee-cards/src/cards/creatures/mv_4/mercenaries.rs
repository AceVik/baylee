//! Mercenaries — {3}{W} — Creature — Human Mercenary
//! Oracle: {3}: The next time this creature would deal damage to you this turn, prevent that damage. Any player may activate this ability.
//! Set: ICE #44 — Ice Age | Scryfall ID: 7b28762d-1ab7-460e-b433-27f5fa858959 | Oracle ID: 9d96671c-7a98-48ec-b479-bc451e18264d
// NOT SUPPORTED: any-player activated ability preventing the next damage this creature would deal to the activating player this turn is not supported; plays as a 3/3 creature with no abilities.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MERCENARIES,
    oracle_id = "9d96671c-7a98-48ec-b479-bc451e18264d",
    scryfall_id = "7b28762d-1ab7-460e-b433-27f5fa858959",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "any-player activated ability preventing the next damage this creature would deal to the activating player this turn is not supported; plays as a 3/3 creature with no abilities",
    ),
    faces = &[face!(
        name = "Mercenaries",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::MERCENARY],
        power = Some(3),
        toughness = Some(3),
    ),],
);
