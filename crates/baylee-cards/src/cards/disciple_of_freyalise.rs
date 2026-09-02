//! Disciple of Freyalise // Garden of Freyalise — {3}{G}{G}{G} — Creature — Elf Druid // Land
//! Set: MH3 #250 — Modern Horizons 3 | Scryfall ID: a8e9ea5a-5e10-4b77-baef-0352ff035483 | Oracle ID: 2699005b-a471-429f-a9d8-fbf2077ee2fd
//! Face: Disciple of Freyalise — {3}{G}{G}{G} — Creature — Elf Druid
//! Face: Garden of Freyalise —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 430,
    oracle_id: "2699005b-a471-429f-a9d8-fbf2077ee2fd",
    scryfall_id: "a8e9ea5a-5e10-4b77-baef-0352ff035483",
    color_identity: ColorSet::from_slice(&[Color::Green]),
    faces: &[
    face! {
        name: "Disciple of Freyalise",
        mana_cost: baylee_core::mana!("{3}{G}{G}{G}"),
        types: TypeSet::CREATURE,
        subtypes: &[subtypes::creature::ELF, subtypes::creature::DRUID],
        power: Some(3),
        toughness: Some(3),
    },
    face! {
        name: "Garden of Freyalise",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
