//! Reverse Polarity — {W}{W} — Instant
//! Oracle: You gain X life, where X is twice the damage dealt to you so far this turn by artifacts.
//! Set: SUM #36 — Summer Magic / Edgar | Scryfall ID: 1e89856a-2496-41b2-b90c-ed42b02980cc | Oracle ID: c7076e2c-81f1-44ad-a4dd-45a01802d364
// IMPLEMENTED — gain twice the damage artifacts have dealt to you so far this turn.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REVERSE_POLARITY,
    oracle_id = "c7076e2c-81f1-44ad-a4dd-45a01802d364",
    scryfall_id = "1e89856a-2496-41b2-b90c-ed42b02980cc",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Reverse Polarity",
        mana_cost = mana!("{W}{W}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(&[Effect::GainLife {
        amount: Amount::Times {
            base: &Amount::DamageThisTurn {
                to: DamageTo::You,
                by: &Filter::ARTIFACT,
                combat: None,
            },
            factor: 2,
        },
    }])],
);
