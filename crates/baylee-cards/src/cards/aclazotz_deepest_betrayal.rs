//! Aclazotz, Deepest Betrayal // Temple of the Dead — {3}{B}{B} — Legendary Creature — Bat God // Land
//! Set: LCI #88 — The Lost Caverns of Ixalan | Scryfall ID: 627c392c-4d18-4eb2-a4e8-c668f61f5487 | Oracle ID: fcdfe9d5-2743-4d3e-ab57-bf0f96beaa15
//! Face: Aclazotz, Deepest Betrayal — {3}{B}{B} — Legendary Creature — Bat God
//! Face: Temple of the Dead —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 205,
    oracle_id: "fcdfe9d5-2743-4d3e-ab57-bf0f96beaa15",
    scryfall_id: "627c392c-4d18-4eb2-a4e8-c668f61f5487",
    color_identity: ColorSet::from_slice(&[Color::Black]),
    commander: CommanderRule::Legendary,
    faces: &[
    face! {
        name: "Aclazotz, Deepest Betrayal",
        mana_cost: baylee_core::mana!("{3}{B}{B}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::creature::BAT, subtypes::creature::GOD],
        power: Some(4),
        toughness: Some(4),
    },
    face! {
        name: "Temple of the Dead",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
