//! Didgeridoo — {1} — Artifact
//! Oracle: {3}: You may put a Minotaur permanent card from your hand onto the battlefield.
//! Set: ME3 #194 — Masters Edition III | Scryfall ID: 8d2ff269-ee43-4684-84a4-ecb4aed3d960 | Oracle ID: e9a2cb10-6526-4a5c-bda7-e2f328b377dc
// IMPLEMENTED — {3}: You may put a Minotaur permanent card from your hand onto the battlefield.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static MINOTAUR_PERMANENT: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::creature::MINOTAUR),
    Filter::Or(&[
        Filter::ARTIFACT,
        Filter::CREATURE,
        Filter::ENCHANTMENT,
        Filter::LAND,
        Filter::PLANESWALKER,
    ]),
]);

card!(
    index = index::DIDGERIDOO,
    oracle_id = "e9a2cb10-6526-4a5c-bda7-e2f328b377dc",
    scryfall_id = "8d2ff269-ee43-4684-84a4-ecb4aed3d960",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Didgeridoo",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{3}"),
        &[Effect::PutFromHandOntoBattlefield {
            tapped: false,
            filter: &MINOTAUR_PERMANENT,
            mana_value: None,
            optional: true,
        }]
    )],
);
