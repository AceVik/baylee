//! Treefolk Seedlings — {2}{G} — Creature — Treefolk
//! Oracle: Treefolk Seedlings's toughness is equal to the number of Forests you control.
//! Set: USG #278 — Urza's Saga | Scryfall ID: 1ec5ee47-c2ef-442d-b1e7-323a8dba6627 | Oracle ID: 87285539-29c8-4cc7-a22c-4a4b96a61e82
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static FOREST: Filter = Filter::And(&[Filter::LAND, Filter::HasSubtype(subtypes::land::FOREST)]);

card!(
    index = index::TREEFOLK_SEEDLINGS,
    oracle_id = "87285539-29c8-4cc7-a22c-4a4b96a61e82",
    scryfall_id = "1ec5ee47-c2ef-442d-b1e7-323a8dba6627",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Treefolk Seedlings",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::TREEFOLK],
        power = Some(2),
        toughness = Some(0),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::CharacteristicPTSeparate {
            power: PtCount::Fixed(2),
            toughness: PtCount::YouControl(&FOREST),
        }
    ),],
);
