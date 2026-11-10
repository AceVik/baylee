//! Rouse — {1}{B} — Instant
//! Oracle: If you control a Swamp, you may pay 2 life rather than pay this spell's mana cost.
//! Oracle: Target creature gets +2/+0 until end of turn.
//! Set: MMQ #157 — Mercadian Masques | Scryfall ID: ad01a8e2-5dc5-49a3-ad1c-7d5bf006b774 | Oracle ID: 475099d9-4743-420a-ada8-3932f5ef9e82
// NOT SUPPORTED: conditional life-payment alternative cost not yet in DSL
// IMPLEMENTED — hand-written with tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ROUSE,
    oracle_id = "475099d9-4743-420a-ada8-3932f5ef9e82",
    scryfall_id = "ad01a8e2-5dc5-49a3-ad1c-7d5bf006b774",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial("conditional life-payment alternative cost not yet in DSL"),
    faces = &[face!(
        name = "Rouse",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::PumpTarget {
            power: Amount::Fixed(2),
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    ),],
);
