//! Great Defender — {W} — Instant
//! Oracle: Target creature gets +0/+X until end of turn, where X is its mana value.
//! Set: LEG #16 — Legends | Scryfall ID: 879a8653-1538-4f78-a3d3-a900a4d9499b | Oracle ID: c84496bc-6421-4930-a118-b0f9ee7e13f6
// IMPLEMENTED — the target creature gets +0/+X, X read off the target's own
// mana value (`Amount::TargetCmc`) through `PumpTarget`.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GREAT_DEFENDER,
    oracle_id = "c84496bc-6421-4930-a118-b0f9ee7e13f6",
    scryfall_id = "879a8653-1538-4f78-a3d3-a900a4d9499b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Great Defender",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::PumpTarget {
            power: Amount::Fixed(0),
            toughness: Amount::TargetCmc,
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
