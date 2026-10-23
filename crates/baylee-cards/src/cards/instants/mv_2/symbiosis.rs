//! Symbiosis — {1}{G} — Instant
//! Oracle: Two target creatures each get +2/+2 until end of turn.
//! Set: USG #275 — Urza's Saga | Scryfall ID: 5d35e2c5-1871-4749-aefa-4a7a69645c03 | Oracle ID: 787b494b-731b-4e99-8e46-157bc86f6eea
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SYMBIOSIS,
    oracle_id = "787b494b-731b-4e99-8e46-157bc86f6eea",
    scryfall_id = "5d35e2c5-1871-4749-aefa-4a7a69645c03",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Symbiosis",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::PumpTarget {
            power: Amount::Fixed(2),
            toughness: Amount::Fixed(2),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::exactly(TargetSpec::Object(&Filter::CREATURE), 2)),
    ),],
);
