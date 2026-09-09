//! Pinnacle Monk // Mystic Peak — {3}{R}{R} — Creature — Djinn Monk // Land
//! Set: MH3 #246 — Modern Horizons 3 | Scryfall ID: 24d4f26e-7f96-4b38-867e-4fac819b2679 | Oracle ID: f3d48efa-910a-4872-a5b1-a353c5dbce99
//! Face: Pinnacle Monk — {3}{R}{R} — Creature — Djinn Monk
//! Face: Mystic Peak —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 862,
    oracle_id: "f3d48efa-910a-4872-a5b1-a353c5dbce99",
    scryfall_id: "24d4f26e-7f96-4b38-867e-4fac819b2679",
    color_identity: ColorSet::from_slice(&[Color::Red]),
    faces: &[
    face! {
        name: "Pinnacle Monk",
        mana_cost: baylee_core::mana!("{3}{R}{R}"),
        types: TypeSet::CREATURE,
        subtypes: &[subtypes::creature::DJINN, subtypes::creature::MONK],
        power: Some(2),
        toughness: Some(2),
    },
    face! {
        name: "Mystic Peak",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
