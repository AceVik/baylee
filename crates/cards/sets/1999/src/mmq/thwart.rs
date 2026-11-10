//! Thwart — {2}{U}{U} — Instant
//! Oracle: You may return three Islands you control to their owner's hand rather than pay this spell's mana cost.
//! Oracle: Counter target spell.
//! Set: MMQ #108 — Mercadian Masques | Scryfall ID: c12a0717-e9ea-4be3-a29f-179671ed4489 | Oracle ID: 615a149a-2869-4b18-bbbe-9ec1825049d7
// NOT SUPPORTED: Alternate cost of returning three Islands you control to their owner's hand is not supported; spell effect works when cast for {2}{U}{U}.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THWART,
    oracle_id = "615a149a-2869-4b18-bbbe-9ec1825049d7",
    scryfall_id = "c12a0717-e9ea-4be3-a29f-179671ed4489",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial("alternate return-three-Islands cost not supported"),
    faces = &[face!(
        name = "Thwart",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::CounterTargetSpell],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
    ),],
);
