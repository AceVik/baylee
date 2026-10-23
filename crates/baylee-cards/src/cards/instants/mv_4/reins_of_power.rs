//! Reins of Power — {2}{U}{U} — Instant
//! Oracle: Untap all creatures you control and all creatures target opponent controls. You and that opponent each gain control of all creatures the other controls until end of turn. Those creatures gain haste until end of turn.
//! Set: CLB #734 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: 9b5730aa-c74a-4c74-bb1b-a133a3c353ec | Oracle ID: 96381388-1e82-411c-8290-1f3e909d3b5f
// PARTIAL: simultaneously exchanging control of all creatures between you and target opponent until end of turn is not supported; targets an opponent, untaps all creatures you and opponents control, and grants them haste until end of turn.

use baylee_cards_dsl::prelude::*;

static YOUR_OR_OPPONENT_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Or(&[Filter::ControlledByYou, Filter::ControlledByOpponent]),
]);

card!(
    index = index::REINS_OF_POWER,
    oracle_id = "96381388-1e82-411c-8290-1f3e909d3b5f",
    scryfall_id = "9b5730aa-c74a-4c74-bb1b-a133a3c353ec",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "simultaneously exchanging control of all creatures between you and target opponent until end of turn is not supported; targets an opponent, untaps all creatures you and opponents control, and grants them haste until end of turn",
    ),
    faces = &[face!(
        name = "Reins of Power",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::UntapAll {
                filter: &YOUR_OR_OPPONENT_CREATURE,
            },
            Effect::PumpFilter {
                filter: &YOUR_OR_OPPONENT_CREATURE,
                controlled_by: None,
                power: Amount::Fixed(0),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::HASTE,
                duration: Duration::UntilEndOfTurn,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    ),],
);
