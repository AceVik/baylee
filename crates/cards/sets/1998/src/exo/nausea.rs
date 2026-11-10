//! Nausea — {1}{B} — Sorcery
//! Oracle: All creatures get -1/-1 until end of turn.
//! Set: EMA #97 — Eternal Masters | Scryfall ID: 2569173f-df5e-4518-9fb3-f972210595df | Oracle ID: 2b8abae2-dd15-41ee-81ae-6a463723b43e
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NAUSEA,
    oracle_id = "2b8abae2-dd15-41ee-81ae-6a463723b43e",
    scryfall_id = "2569173f-df5e-4518-9fb3-f972210595df",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Nausea",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::PumpFilter {
        filter: &Filter::CREATURE,
        controlled_by: None,
        power: Amount::NegXFixed(1),
        toughness: Amount::NegXFixed(1),
        keywords: KeywordSet::EMPTY,
        duration: Duration::UntilEndOfTurn
    }]),],
);
