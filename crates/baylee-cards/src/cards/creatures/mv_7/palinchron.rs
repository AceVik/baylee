//! Palinchron — {5}{U}{U} — Creature — Illusion
//! Oracle: Flying
//! Oracle: When this creature enters, untap up to seven lands.
//! Oracle: {2}{U}{U}: Return this creature to its owner's hand.
//! Set: VMA #86 — Vintage Masters | Scryfall ID: ef772879-44aa-428d-8c12-50d38f8bac80 | Oracle ID: 0023888e-7bec-43e0-8dee-d1a4eb94b372
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PALINCHRON,
    oracle_id = "0023888e-7bec-43e0-8dee-d1a4eb94b372",
    scryfall_id = "ef772879-44aa-428d-8c12-50d38f8bac80",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Palinchron",
        mana_cost = mana!("{5}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION],
        power = Some(4),
        toughness = Some(5),
        keywords = KeywordSet::FLYING,
    ),],
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::UntapChosen {
                filter: &Filter::LAND,
                count: 7,
            }],
        ),
        activated!(
            cost!("{2}{U}{U}"),
            &[Effect::bounce(TargetSpec::ThisObject)],
        ),
    ],
);
