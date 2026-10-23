//! Rank and File — {2}{B}{B} — Creature — Zombie
//! Oracle: When this creature enters, green creatures get -1/-1 until end of turn.
//! Set: ULG #65 — Urza's Legacy | Scryfall ID: 59481cb5-2cb0-4b8c-84ee-519399862d46 | Oracle ID: c0b8ab40-3c27-48bd-ae19-fb1a155a0522
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static PUMP_ALL1: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Green])),
]);

card!(
    index = index::RANK_AND_FILE,
    oracle_id = "c0b8ab40-3c27-48bd-ae19-fb1a155a0522",
    scryfall_id = "59481cb5-2cb0-4b8c-84ee-519399862d46",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Rank and File",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::PumpFilter {
            filter: &PUMP_ALL1,
            controlled_by: None,
            power: Amount::NegXFixed(1),
            toughness: Amount::NegXFixed(1),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn
        }]
    ),],
);
