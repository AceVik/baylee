//! Sick and Tired — {2}{B} — Instant
//! Oracle: Two target creatures each get -1/-1 until end of turn.
//! Set: ULG #66 — Urza's Legacy | Scryfall ID: 8736f8a2-ee8d-49d2-883f-b22cbe3f3645 | Oracle ID: 070d82db-0851-4f6d-8bf7-1a79c4c2cfee
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SICK_AND_TIRED,
    oracle_id = "070d82db-0851-4f6d-8bf7-1a79c4c2cfee",
    scryfall_id = "8736f8a2-ee8d-49d2-883f-b22cbe3f3645",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Sick and Tired",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::PumpTarget {
            power: Amount::NegXFixed(1),
            toughness: Amount::NegXFixed(1),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::exactly(TargetSpec::Object(&Filter::CREATURE), 2,)),
    )],
);
