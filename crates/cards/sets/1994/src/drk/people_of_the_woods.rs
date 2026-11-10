//! People of the Woods — {G}{G} — Creature — Human
//! Oracle: People of the Woods's toughness is equal to the number of Forests you control.
//! Set: DRK #83 — The Dark | Scryfall ID: 2fb5926f-9988-4bc0-b2b7-e286db208310 | Oracle ID: 66e7a760-2b2f-4af9-983c-300c657beaaf

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static FOREST: Filter = Filter::HasSubtype(subtypes::land::FOREST);

card!(
    index = index::PEOPLE_OF_THE_WOODS,
    oracle_id = "66e7a760-2b2f-4af9-983c-300c657beaaf",
    scryfall_id = "2fb5926f-9988-4bc0-b2b7-e286db208310",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "People of the Woods",
        mana_cost = mana!("{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(1),
        toughness = Some(0),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::CharacteristicPTSeparate {
            power: PtCount::Fixed(1),
            toughness: PtCount::YouControl(&FOREST),
        }
    )],
);
