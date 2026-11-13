//! Piety — {2}{W} — Instant
//! Oracle: Blocking creatures get +0/+3 until end of turn.
//! Set: 4ED #41 — Fourth Edition | Scryfall ID: d4942a9f-6b8f-438b-a2ea-366228038ed8 | Oracle ID: 0c017406-7fc3-4701-93ec-ddb02044c12a
// IMPLEMENTED — every creature blocking as this resolves gets +0/+3 until end of turn.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PIETY,
    oracle_id = "0c017406-7fc3-4701-93ec-ddb02044c12a",
    scryfall_id = "d4942a9f-6b8f-438b-a2ea-366228038ed8",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Piety",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(&[Effect::PumpFilter {
        filter: &Filter::Blocking,
        controlled_by: None,
        power: Amount::Fixed(0),
        toughness: Amount::Fixed(3),
        keywords: KeywordSet::EMPTY,
        duration: Duration::UntilEndOfTurn,
    }])],
);
