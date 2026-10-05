//! Shield Wall — {1}{W} — Instant
//! Oracle: Creatures you control get +0/+2 until end of turn.
//! Set: 7ED #44 — Seventh Edition | Scryfall ID: d4b70c30-dbc9-4d30-81d8-b0bde9b626df | Oracle ID: 8148eaa4-6fde-41f2-9b87-ccfc4d8e822f
// IMPLEMENTED — each creature you control gets +0/+2 until end of turn.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SHIELD_WALL,
    oracle_id = "8148eaa4-6fde-41f2-9b87-ccfc4d8e822f",
    scryfall_id = "d4b70c30-dbc9-4d30-81d8-b0bde9b626df",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Shield Wall",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::PumpFilter {
        filter: &Filter::YOUR_CREATURE,
        controlled_by: None,
        power: Amount::Fixed(0),
        toughness: Amount::Fixed(2),
        keywords: KeywordSet::EMPTY,
        duration: Duration::UntilEndOfTurn,
    }])],
);
