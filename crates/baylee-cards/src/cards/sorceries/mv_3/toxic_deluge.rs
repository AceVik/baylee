//! Toxic Deluge — {2}{B} — Sorcery
//! Oracle: As an additional cost to cast this spell, pay X life.
//! Oracle: All creatures get -X/-X until end of turn.
//! Set: MSC #161 — Marvel Super Heroes Commander | Scryfall ID: de5afccc-8d42-4bd6-b068-b9ea2361655e | Oracle ID: afaef788-34d1-460b-b884-9d7ae6ddeb18
// IMPLEMENTED — mandatory pay-life-X additional cost + X-driven global
// debuff until end of turn.

use baylee_cards_dsl::prelude::*;

card! {
    index: 172,
    oracle_id: "afaef788-34d1-460b-b884-9d7ae6ddeb18",
    scryfall_id: "de5afccc-8d42-4bd6-b068-b9ea2361655e",
    faces: &[face! {
        name: "Toxic Deluge",
        mana_cost: baylee_core::mana!("{2}{B}"),
        types: TypeSet::SORCERY,
        mandatory_additional_costs: &[CostPart::PayLifeX],
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[Effect::PumpFilter {
            filter: &Filter::CREATURE,
            power: Amount::NegX,
            toughness: Amount::NegX,
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }])],
}

// X life paid at cast; all creatures get -X/-X until end of turn.
