//! Ojer Kaslem, Deepest Growth // Temple of Cultivation — {3}{G}{G} — Legendary Creature — God // Land
//! Set: LCI #204 — The Lost Caverns of Ixalan | Scryfall ID: 0cbc43a3-8cba-4988-9de1-c89aedd79ada | Oracle ID: eda11077-b2ce-408b-b982-def2da8fe599
//! Face: Ojer Kaslem, Deepest Growth — {3}{G}{G} — Legendary Creature — God
//! Face: Temple of Cultivation —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 827,
    oracle_id: "eda11077-b2ce-408b-b982-def2da8fe599",
    scryfall_id: "0cbc43a3-8cba-4988-9de1-c89aedd79ada",
    color_identity: ColorSet::from_slice(&[Color::Green]),
    commander: CommanderRule::Legendary,
    faces: &[
    face! {
        name: "Ojer Kaslem, Deepest Growth",
        mana_cost: baylee_core::mana!("{3}{G}{G}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::creature::GOD],
        power: Some(6),
        toughness: Some(5),
    },
    face! {
        name: "Temple of Cultivation",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
