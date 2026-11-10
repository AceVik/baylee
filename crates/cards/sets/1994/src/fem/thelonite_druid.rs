//! Thelonite Druid — {2}{G} — Creature — Human Cleric Druid
//! Oracle: {1}{G}, {T}, Sacrifice a creature: Forests you control become 2/3 creatures until end of turn. They're still lands.
//! Set: ME2 #182 — Masters Edition II | Scryfall ID: 644aa4e7-9ea4-454b-abce-2a1a53c25560 | Oracle ID: adbc8970-1bbc-4f30-9a2e-eebaf074eae6
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const YOUR_FOREST: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::land::FOREST),
    Filter::ControlledByYou,
]);

card!(
    index = index::THELONITE_DRUID,
    oracle_id = "adbc8970-1bbc-4f30-9a2e-eebaf074eae6",
    scryfall_id = "644aa4e7-9ea4-454b-abce-2a1a53c25560",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Thelonite Druid",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::HUMAN,
            subtypes::creature::CLERIC,
            subtypes::creature::DRUID,
        ],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{1}{G}", TapSelf, Sacrifice(&Filter::YOUR_CREATURE)),
        &[
            Effect::continuous(
                &YOUR_FOREST,
                Modifier::AddType(TypeSet::CREATURE),
                Duration::UntilEndOfTurn,
            ),
            Effect::continuous(
                &YOUR_FOREST,
                Modifier::SetPT(2, 3),
                Duration::UntilEndOfTurn
            ),
        ],
    ),],
);
