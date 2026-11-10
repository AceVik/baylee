//! Crossbow Ambush — {G} — Instant
//! Oracle: Creatures you control gain reach until end of turn. (They can block creatures with flying.)
//! Set: STH #105 — Stronghold | Scryfall ID: 283146fd-2307-4019-b23d-7fb1893dc46c | Oracle ID: c10fd7a6-360f-43d6-a4ef-d990c97cb2f6
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CROSSBOW_AMBUSH,
    oracle_id = "c10fd7a6-360f-43d6-a4ef-d990c97cb2f6",
    scryfall_id = "283146fd-2307-4019-b23d-7fb1893dc46c",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Crossbow Ambush",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::PumpFilter {
        filter: &Filter::YOUR_CREATURE,
        controlled_by: None,
        power: Amount::Fixed(0),
        toughness: Amount::Fixed(0),
        keywords: KeywordSet::REACH,
        duration: Duration::UntilEndOfTurn
    }]),],
);
