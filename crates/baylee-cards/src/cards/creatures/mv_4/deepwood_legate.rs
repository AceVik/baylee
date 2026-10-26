//! Deepwood Legate — {3}{B} — Creature — Shade
//! Oracle: If an opponent controls a Forest and you control a Swamp, you may cast this spell without paying its mana cost.
//! Oracle: {B}: This creature gets +1/+1 until end of turn.
//! Set: MMQ #132 — Mercadian Masques | Scryfall ID: 54f01925-7fd0-472d-91a4-3309e615f22f | Oracle ID: 414eb407-dd39-41d2-bf25-a7a77ff35cb4
// PARTIAL — 1/1 Shade with {B}: +1/+1 until end of turn.
// NOT SUPPORTED: "If an opponent controls a Forest and you control a Swamp, you may cast this spell without paying its mana cost" — `AltCondition` has no condition checking both an opponent's Forest and your Swamp.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DEEPWOOD_LEGATE,
    oracle_id = "414eb407-dd39-41d2-bf25-a7a77ff35cb4",
    scryfall_id = "54f01925-7fd0-472d-91a4-3309e615f22f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "conditional free cast (opponent controls Forest and you control Swamp) not supported"
    ),
    faces = &[face!(
        name = "Deepwood Legate",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SHADE],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{B}"),
        &[Effect::PumpFilter {
            filter: &Filter::This,
            controlled_by: None,
            power: Amount::Fixed(1),
            toughness: Amount::Fixed(1),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
    ),],
);
