//! Smite — {W} — Instant
//! Oracle: Destroy target blocked creature.
//! Set: TPR #28 — Tempest Remastered | Scryfall ID: b2286067-c964-4bed-bfb8-c6280f104b63 | Oracle ID: 367379f2-e1e4-48e7-b2c0-1003de55adbe
// PARTIAL: a dedicated Filter::Blocked variant to prevent targeting an attacker during the declare attackers step before blockers are declared is not yet modeled; destroys target blocked attacking creature.

use baylee_cards_dsl::prelude::*;

static BLOCKED_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Attacking,
    Filter::Not(&Filter::Unblocked),
]);

card!(
    index = index::SMITE,
    oracle_id = "367379f2-e1e4-48e7-b2c0-1003de55adbe",
    scryfall_id = "b2286067-c964-4bed-bfb8-c6280f104b63",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "a dedicated Filter::Blocked variant to prevent targeting an attacker during the declare attackers step before blockers are declared is not yet modeled; destroys target blocked attacking creature",
    ),
    faces = &[face!(
        name = "Smite",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::destroy(TargetSpec::Object(&BLOCKED_CREATURE))],
        targets = Some(TargetReq::one(TargetSpec::Object(&BLOCKED_CREATURE))),
    ),],
);
