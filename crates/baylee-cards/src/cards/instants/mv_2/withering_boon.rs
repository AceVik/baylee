//! Withering Boon — {1}{B} — Instant
//! Oracle: As an additional cost to cast this spell, pay 3 life.
//! Oracle: Counter target creature spell.
//! Set: MIR #152 — Mirage | Scryfall ID: 6e6499cb-6073-4c94-8c82-47f489094df5 | Oracle ID: 6d87415b-0618-48e1-9510-320c4611ecda
// IMPLEMENTED — additional cost paying 3 life and countering target creature spell.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WITHERING_BOON,
    oracle_id = "6d87415b-0618-48e1-9510-320c4611ecda",
    scryfall_id = "6e6499cb-6073-4c94-8c82-47f489094df5",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Withering Boon",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::INSTANT,
        mandatory_additional_costs = &[CostPart::PayLife(3)],
    ),],
    abilities = &[spell!(
        &[Effect::CounterTargetSpell],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::CREATURE))),
    )],
);
