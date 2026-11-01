//! Temper — {X}{1}{W} — Instant
//! Oracle: Prevent the next X damage that would be dealt to target creature this turn. For each 1 damage prevented this way, put a +1/+1 counter on that creature.
//! Set: STH #20 — Stronghold | Scryfall ID: e4bff910-24ab-45a0-b014-30bef6263c38 | Oracle ID: 9cefff18-96b1-4090-93cc-1ca981a58fbd
// PARTIAL: putting a +1/+1 counter on target creature for each 1 damage prevented this way is not supported; prevents the next X damage that would be dealt to target creature this turn.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TEMPER,
    oracle_id = "9cefff18-96b1-4090-93cc-1ca981a58fbd",
    scryfall_id = "e4bff910-24ab-45a0-b014-30bef6263c38",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "putting a +1/+1 counter on target creature for each 1 damage prevented this way is not supported; prevents the next X damage that would be dealt to target creature this turn",
    ),
    faces = &[face!(
        name = "Temper",
        mana_cost = mana!("{X}{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::PreventNextDamage {
            target: TargetSpec::Object(&Filter::CREATURE),
            amount: Amount::X,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    ),],
);
