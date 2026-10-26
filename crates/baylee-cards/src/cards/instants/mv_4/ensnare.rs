//! Ensnare — {3}{U} — Instant
//! Oracle: You may return two Islands you control to their owner's hand rather than pay this spell's mana cost.
//! Oracle: Tap all creatures.
//! Set: NEM #32 — Nemesis | Scryfall ID: 055b344a-4eb1-4579-ac50-973b18e12fad | Oracle ID: df197279-ddb2-4237-8bb1-8df41d3bed9e
// NOT SUPPORTED: taps all creatures for {3}{U}; the alternative cost of returning two Islands you control to their owner's hand is not yet modeled
// PARTIAL: taps all creatures for {3}{U}; the alternative cost of returning two Islands you control to their owner's hand is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ENSNARE,
    oracle_id = "df197279-ddb2-4237-8bb1-8df41d3bed9e",
    scryfall_id = "055b344a-4eb1-4579-ac50-973b18e12fad",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "taps all creatures for {3}{U}; the alternative cost of returning two Islands you control to their owner's hand is not yet modeled",
    ),
    faces = &[face!(
        name = "Ensnare",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::TapAll {
        filter: &Filter::CREATURE,
    }]),],
);
