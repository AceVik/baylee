//! Hunger of the Nim — {1}{B} — Sorcery
//! Oracle: Target creature gets +1/+0 until end of turn for each artifact you control.
//! Set: DST #46 — Darksteel | Scryfall ID: bba50e47-2417-447f-b334-50ef0faecfae | Oracle ID: 1af3c6ff-2884-4d8c-a01f-6f74d8ea10cc
// IMPLEMENTED — one counted pump on the target: +1/+0 until end of turn for
// each artifact you control (Amount::CountOf over Filter::YOUR_ARTIFACT on the
// battlefield).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HUNGER_OF_THE_NIM,
    oracle_id = "1af3c6ff-2884-4d8c-a01f-6f74d8ea10cc",
    scryfall_id = "bba50e47-2417-447f-b334-50ef0faecfae",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hunger of the Nim",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::PumpTarget {
            power: Amount::CountOf {
                filter: &Filter::YOUR_ARTIFACT,
                zone: ZoneSel::Battlefield,
            },
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )],
);
