//! Ojer Pakpatiq, Deepest Epoch // Temple of Cyclical Time — {2}{U}{U} — Legendary Creature — God // Land
//! Set: LCI #67 — The Lost Caverns of Ixalan | Scryfall ID: a9d71007-bc04-4dff-ad3f-e2c0b5b4400e | Oracle ID: 34ef174e-1b3d-43d5-9f72-3d35befbdd7f
//! Face: Ojer Pakpatiq, Deepest Epoch — {2}{U}{U} — Legendary Creature — God
//! Face: Temple of Cyclical Time —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 828,
    oracle_id: "34ef174e-1b3d-43d5-9f72-3d35befbdd7f",
    scryfall_id: "a9d71007-bc04-4dff-ad3f-e2c0b5b4400e",
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    commander: CommanderRule::Legendary,
    faces: &[
    face! {
        name: "Ojer Pakpatiq, Deepest Epoch",
        mana_cost: baylee_core::mana!("{2}{U}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::creature::GOD],
        power: Some(4),
        toughness: Some(3),
    },
    face! {
        name: "Temple of Cyclical Time",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
