//! Dregs of Sorrow — {X}{4}{B} — Sorcery
//! Oracle: Destroy X target nonblack creatures. Draw X cards.
//! Set: C14 #143 — Commander 2014 | Scryfall ID: 3c004cd1-9190-4356-ad68-5012c7ef63de | Oracle ID: 8126e60b-cf3d-4bf0-bfb5-f1e914d3e20e
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

const NONBLACK_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::Black]))),
]);

card!(
    index = index::DREGS_OF_SORROW,
    oracle_id = "8126e60b-cf3d-4bf0-bfb5-f1e914d3e20e",
    scryfall_id = "3c004cd1-9190-4356-ad68-5012c7ef63de",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dregs of Sorrow",
        mana_cost = mana!("{X}{4}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[
            Effect::destroy(TargetSpec::Object(&NONBLACK_CREATURE)),
            Effect::DrawCards { amount: Amount::X },
        ],
        targets = Some(TargetReq::x_targets(TargetSpec::Object(&NONBLACK_CREATURE))),
    )],
);
