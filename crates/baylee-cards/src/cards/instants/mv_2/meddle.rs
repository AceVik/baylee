//! Meddle — {1}{U} — Instant
//! Oracle: If target spell has only one target and that target is a creature, change that spell's target to another creature.
//! Set: ONS #92 — Onslaught | Scryfall ID: 685edfe8-9770-47c6-95fb-0816f3126f04 | Oracle ID: 67d6fbcb-2df7-4b5a-8464-158f6b720a2e
// NOT SUPPORTED: checking that the targeted spell's single target is currently a creature is not in Filter or Condition.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MEDDLE,
    oracle_id = "67d6fbcb-2df7-4b5a-8464-158f6b720a2e",
    scryfall_id = "685edfe8-9770-47c6-95fb-0816f3126f04",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "targets a spell with a single target and changes its target to another creature; checking that the spell's current target is a creature is not in Filter or Condition",
    ),
    faces = &[face!(
        name = "Meddle",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::ChangeTarget {
            to: &Filter::CREATURE,
        }],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::WithSingleTarget))),
    )],
);
