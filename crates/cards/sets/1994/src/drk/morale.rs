//! Morale — {1}{W}{W} — Instant
//! Oracle: Attacking creatures get +1/+1 until end of turn.
//! Set: 4ED #36 — Fourth Edition | Scryfall ID: b52cdfe3-fca1-420c-b5c0-2366ea58345b | Oracle ID: 8af9d623-efa3-4cbc-a903-aaec32d78b0a

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MORALE,
    oracle_id = "8af9d623-efa3-4cbc-a903-aaec32d78b0a",
    scryfall_id = "b52cdfe3-fca1-420c-b5c0-2366ea58345b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Morale",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::CreateContinuousEffect {
        layer: Layer::PtModify,
        filter: &Filter::ATTACKING_CREATURE,
        modifier: Modifier::ModifyPT(1, 1),
        duration: Duration::UntilEndOfTurn,
    }])],
);
