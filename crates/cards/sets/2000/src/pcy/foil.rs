//! Foil — {2}{U}{U} — Instant
//! Oracle: You may discard an Island card and another card rather than pay this spell's mana cost.
//! Oracle: Counter target spell.
//! Set: UMA #55 — Ultimate Masters | Scryfall ID: e8b39fd6-9240-4f76-b12c-e7d9aa88f061 | Oracle ID: 7dde4eb6-e9d7-4259-abc2-3af738e0f00f
// NOT SUPPORTED: counters target spell for {2}{U}{U}; the alternative cost of discarding an Island card and another card is not yet modeled
// PARTIAL: counters target spell for {2}{U}{U}; the alternative cost of discarding an Island card and another card is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FOIL,
    oracle_id = "7dde4eb6-e9d7-4259-abc2-3af738e0f00f",
    scryfall_id = "e8b39fd6-9240-4f76-b12c-e7d9aa88f061",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "counters target spell for {2}{U}{U}; the alternative cost of discarding an Island card and another card is not yet modeled",
    ),
    faces = &[face!(
        name = "Foil",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::CounterTargetSpell],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
    ),],
);
