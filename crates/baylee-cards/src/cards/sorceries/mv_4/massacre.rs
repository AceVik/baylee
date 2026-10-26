//! Massacre — {2}{B}{B} — Sorcery
//! Oracle: If an opponent controls a Plains and you control a Swamp, you may cast this spell without paying its mana cost.
//! Oracle: All creatures get -2/-2 until end of turn.
//! Set: NEM #58 — Nemesis | Scryfall ID: f05f5d93-50d1-4aa6-af05-383a6808345b | Oracle ID: ce1866e9-28de-4bcc-9999-11958e6c13a6
// PARTIAL — all creatures get -2/-2 until end of turn.
// NOT SUPPORTED: conditional free alternative cost when an opponent controls a Plains and you control a Swamp (`AltCondition` has no two-player land-subtype conjunction).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MASSACRE,
    oracle_id = "ce1866e9-28de-4bcc-9999-11958e6c13a6",
    scryfall_id = "f05f5d93-50d1-4aa6-af05-383a6808345b",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "AltCondition has no two-player land-subtype conjunction for free casting when an opponent controls a Plains and you control a Swamp",
    ),
    faces = &[face!(
        name = "Massacre",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::PumpFilter {
        filter: &Filter::CREATURE,
        controlled_by: None,
        power: Amount::NegXFixed(2),
        toughness: Amount::NegXFixed(2),
        keywords: KeywordSet::EMPTY,
        duration: Duration::UntilEndOfTurn,
    }]),],
);
