//! Rebound — {1}{U} — Instant
//! Oracle: Change the target of target spell that targets only a player. The new target must be a player.
//! Set: STH #40 — Stronghold | Scryfall ID: bb6ca66e-1116-4739-8375-87af99e9bba5 | Oracle ID: 9769ed80-b546-49f5-9329-689e619dabdd
// PARTIAL: restricting the targeted single-target spell and its new target specifically to players is not supported; targets a spell with a single target and changes its target to any legal target.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REBOUND,
    oracle_id = "9769ed80-b546-49f5-9329-689e619dabdd",
    scryfall_id = "bb6ca66e-1116-4739-8375-87af99e9bba5",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "restricting the targeted single-target spell and its new target specifically to players is not supported; targets a spell with a single target and changes its target to any legal target",
    ),
    faces = &[face!(
        name = "Rebound",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::ChangeTarget { to: &Filter::Any }],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::WithSingleTarget))),
    ),],
);
