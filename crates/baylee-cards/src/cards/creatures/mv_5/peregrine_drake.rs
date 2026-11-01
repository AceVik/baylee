//! Peregrine Drake — {4}{U} — Creature — Drake
//! Oracle: Flying
//! Oracle: When this creature enters, untap up to five lands.
//! Set: USG #88 — Urza's Saga | Scryfall ID: 4951863f-1c16-4d09-ba9a-f57dc3d81a20 | Oracle ID: 0bd67481-6bd9-48d6-92bd-8933b5ea1eae
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PEREGRINE_DRAKE,
    oracle_id = "0bd67481-6bd9-48d6-92bd-8933b5ea1eae",
    scryfall_id = "4951863f-1c16-4d09-ba9a-f57dc3d81a20",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Peregrine Drake",
        mana_cost = mana!("{4}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRAKE],
        power = Some(2),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::UntapChosen {
            filter: &Filter::LAND,
            count: 5,
        }],
    )],
);
