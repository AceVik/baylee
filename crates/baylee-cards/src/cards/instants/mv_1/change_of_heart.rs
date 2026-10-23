//! Change of Heart — {W} — Instant
//! Oracle: Buyback {3} (You may pay an additional {3} as you cast this spell. If you do, put this card into your hand as it resolves.)
//! Oracle: Target creature can't attack this turn.
//! Set: STH #3 — Stronghold | Scryfall ID: afaf277e-b430-4c96-880c-ae654973478c | Oracle ID: a8c0b41b-916c-4da9-9c39-a058c91a78f4
// PARTIAL: target creature can't attack this turn; buyback {3} is not supported

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CHANGE_OF_HEART,
    oracle_id = "a8c0b41b-916c-4da9-9c39-a058c91a78f4",
    scryfall_id = "afaf277e-b430-4c96-880c-ae654973478c",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage =
        Coverage::Partial("target creature can't attack this turn; buyback {3} is not supported"),
    faces = &[face!(
        name = "Change of Heart",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::ModifyTargets {
            modifier: Modifier::CantAttack,
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
