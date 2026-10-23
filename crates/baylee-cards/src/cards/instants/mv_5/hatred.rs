//! Hatred — {3}{B}{B} — Instant
//! Oracle: As an additional cost to cast this spell, pay X life.
//! Oracle: Target creature gets +X/+0 until end of turn.
//! Set: EXO #64 — Exodus | Scryfall ID: 2383a8d9-96fd-4f9a-bcf9-eb81fdb15ead | Oracle ID: 40601061-1b25-4db8-8b99-33324ce945cf
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HATRED,
    oracle_id = "40601061-1b25-4db8-8b99-33324ce945cf",
    scryfall_id = "2383a8d9-96fd-4f9a-bcf9-eb81fdb15ead",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hatred",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::INSTANT,
        mandatory_additional_costs = &[CostPart::PayLifeX],
    ),],
    abilities = &[spell!(
        &[Effect::PumpTarget {
            power: Amount::X,
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
