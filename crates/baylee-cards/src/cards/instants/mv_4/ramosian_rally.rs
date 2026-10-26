//! Ramosian Rally — {3}{W} — Instant
//! Oracle: If you control a Plains, you may tap an untapped creature you control rather than pay this spell's mana cost.
//! Oracle: Creatures you control get +1/+1 until end of turn.
//! Set: MMQ #38 — Mercadian Masques | Scryfall ID: 7fc0ff04-43e7-4a0d-b7e2-8bab72cc6cc0 | Oracle ID: 2731c237-eda1-4b74-a294-502d574f4435
// NOT SUPPORTED: optional alternative cost (tapping an untapped creature you control if you control a Plains) not supported
// PARTIAL — omits optional alternative cost (tapping an untapped creature you control if you control a Plains).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RAMOSIAN_RALLY,
    oracle_id = "2731c237-eda1-4b74-a294-502d574f4435",
    scryfall_id = "7fc0ff04-43e7-4a0d-b7e2-8bab72cc6cc0",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "optional alternative cost (tapping an untapped creature you control if you control a Plains) not supported"
    ),
    faces = &[face!(
        name = "Ramosian Rally",
        mana_cost = mana!("{3}{W}"),
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
