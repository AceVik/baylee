//! Exile — {2}{W} — Instant
//! Oracle: Exile target nonwhite attacking creature. You gain life equal to its toughness.
//! Set: VMA #27 — Vintage Masters | Scryfall ID: cba8fb8d-fdde-4e4b-9dc7-feaecf8ed222 | Oracle ID: 8c91d50b-bf58-4d06-b3b7-4b6db867bfce

use baylee_cards_dsl::prelude::*;

static NONWHITE_ATTACKING_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Attacking,
    Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::White]))),
]);

card!(
    index = index::EXILE,
    oracle_id = "8c91d50b-bf58-4d06-b3b7-4b6db867bfce",
    scryfall_id = "cba8fb8d-fdde-4e4b-9dc7-feaecf8ed222",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Exile",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::exile(TargetSpec::Object(&NONWHITE_ATTACKING_CREATURE)),
            Effect::GainLife {
                amount: Amount::TargetToughness,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &NONWHITE_ATTACKING_CREATURE,
        ))),
    )],
);
