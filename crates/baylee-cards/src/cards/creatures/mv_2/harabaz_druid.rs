//! Harabaz Druid — {1}{G} — Creature — Human Druid Ally
//! Oracle: {T}: Add X mana of any one color, where X is the number of Allies you control.
//! Set: WWK #105 — Worldwake | Scryfall ID: 78a538cf-2291-49aa-8429-17d97d454479 | Oracle ID: ead985ec-f29f-4a3b-b8b1-061142cc5bd1
// IMPLEMENTED — dynamic Ally mana (choose a color, X = Allies).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static ALLIES_YOU: Filter =
    Filter::And(&[Filter::ControlledByYou, Filter::HasSubtype(creature::ALLY)]);

card! {
    index: 66,
    oracle_id: "ead985ec-f29f-4a3b-b8b1-061142cc5bd1",
    scryfall_id: "78a538cf-2291-49aa-8429-17d97d454479",
    faces: &[face! {
        name: "Harabaz Druid",
        mana_cost: baylee_core::mana!("{1}{G}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::HUMAN, creature::DRUID, creature::ALLY],
        power: Some(0),
        toughness: Some(1),
    }],
    color_identity: ColorSet::from_slice(&[Color::Green]),
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana_combination(
            &[
                ManaColor::White,
                ManaColor::Blue,
                ManaColor::Black,
                ManaColor::Red,
                ManaColor::Green,
            ],
            Amount::CountOf {
                filter: &ALLIES_YOU,
                zone: ZoneSel::Battlefield,
            },
        )])],
}

// X = Allies is delivered by the mana effect.s dynamic Amount::CountOf
// (evaluated at resolution against your battlefield).
