//! Dryad Arbor — (no cost) — Land Creature — Forest Dryad
//! Oracle: (This land isn't a spell, it's affected by summoning sickness, and it has "{T}: Add {G}.")
//! Set: DSC #273 — Duskmourn: House of Horror Commander | Scryfall ID: e3ddbebf-72cd-4d1b-ba0d-d94934654ab7 | Oracle ID: e996cd67-739c-40f4-b276-0042acf26c71
// IMPLEMENTED — Forest dryad creature land with {T}: Add {G}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DRYAD_ARBOR,
    oracle_id = "e996cd67-739c-40f4-b276-0042acf26c71",
    scryfall_id = "e3ddbebf-72cd-4d1b-ba0d-d94934654ab7",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dryad Arbor",
        types = TypeSet::LAND.union(TypeSet::CREATURE),
        subtypes = &[subtypes::land::FOREST, subtypes::creature::DRYAD],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])],
);
