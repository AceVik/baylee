//! Ixidor, Reality Sculptor — {3}{U}{U} — Legendary Creature — Human Wizard
//! Oracle: Face-down creatures get +1/+1.
//! Oracle: {2}{U}: Turn target face-down creature face up.
//! Set: ONS #89 — Onslaught | Scryfall ID: 314d5e89-55f7-42b4-af19-d4d0f499a265 | Oracle ID: 90b98aa7-c8d0-4853-887c-164f8160c3f3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::IXIDOR_REALITY_SCULPTOR,
    oracle_id = "90b98aa7-c8d0-4853-887c-164f8160c3f3",
    scryfall_id = "314d5e89-55f7-42b4-af19-d4d0f499a265",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Ixidor, Reality Sculptor",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(3),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
