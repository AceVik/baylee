//! Spiny Starfish — {2}{U} — Creature — Starfish
//! Oracle: {U}: Regenerate this creature.
//! Oracle: At the beginning of each end step, if this creature regenerated this turn, create a 0/1 blue Starfish creature token for each time it regenerated this turn.
//! Set: ME3 #50 — Masters Edition III | Scryfall ID: b072ccea-816a-4176-9a77-478f207a9a1e | Oracle ID: 663d6c61-e269-4f2f-9d8f-49c4c23f05d8

// PARTIAL — {U}: Regenerate this creature.
// NOT SUPPORTED: "At the beginning of each end step, if this creature regenerated this turn, create a 0/1 blue Starfish creature token for each time it regenerated this turn." Nothing counts how many times a creature regenerated this turn, so no token is created.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SPINY_STARFISH,
    oracle_id = "663d6c61-e269-4f2f-9d8f-49c4c23f05d8",
    scryfall_id = "b072ccea-816a-4176-9a77-478f207a9a1e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Spiny Starfish",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::STARFISH],
        power = Some(0),
        toughness = Some(1),
    ),],
    coverage = Coverage::Partial(
        "The {U} regeneration ability is implemented; the end-step trigger that creates 0/1 blue Starfish tokens for each time this creature regenerated this turn is not implemented."
    ),
    abilities = &[activated!(
        cost!("{U}"),
        &[Effect::Regenerate {
            target: TargetSpec::ThisObject,
        }]
    )],
);
