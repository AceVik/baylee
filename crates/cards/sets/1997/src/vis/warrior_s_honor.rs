//! Warrior's Honor — {2}{W} — Instant
//! Oracle: Creatures you control get +1/+1 until end of turn.
//! Set: 10E #58 — Tenth Edition | Scryfall ID: 09117d06-79e6-4f86-ba92-d1f8fe165147 | Oracle ID: c3c9da49-ef4f-48c7-99c8-0f226b485285
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WARRIOR_S_HONOR,
    oracle_id = "c3c9da49-ef4f-48c7-99c8-0f226b485285",
    scryfall_id = "09117d06-79e6-4f86-ba92-d1f8fe165147",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Warrior's Honor",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::PumpFilter {
        filter: &Filter::YOUR_CREATURE,
        controlled_by: None,
        power: Amount::Fixed(1),
        toughness: Amount::Fixed(1),
        keywords: KeywordSet::EMPTY,
        duration: Duration::UntilEndOfTurn,
    }]),],
);
