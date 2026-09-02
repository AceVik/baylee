//! Force of Will — {3}{U}{U} — Instant
//! Oracle: You may pay 1 life and exile a blue card from your hand rather than pay this spell's mana cost.
//! Oracle: Counter target spell.
//! Set: DMR #50 — Dominaria Remastered | Scryfall ID: 89f612d6-7c59-4a7b-a87d-45f789e88ba5 | Oracle ID: 956381ba-6d37-4a8a-846c-bad79222dbee
// IMPLEMENTED — hard counter with pitch alternative (1 life + exile a blue
// card from hand) via the casting wizard.

static BLUE_CARD: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Blue]));

use baylee_cards_dsl::prelude::*;

card! {
    index: 54,
    oracle_id: "956381ba-6d37-4a8a-846c-bad79222dbee",
    scryfall_id: "89f612d6-7c59-4a7b-a87d-45f789e88ba5",
    faces: &[face! {
        name: "Force of Will",
        mana_cost: baylee_core::mana!("{3}{U}{U}"),
        types: TypeSet::INSTANT,
        alternative_costs: &[AlternativeCost {
            cost: Cost {
                mana: ManaCost::ZERO,
                parts: &[CostPart::PayLife(1), CostPart::ExileFromHand(&BLUE_CARD)],
            },
            condition: AltCondition::Always,
        }],
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[Effect::CounterTargetSpell], targets: Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))))],
}

// Engine-level coverage in baylee-engine s7 tests: pitching (life +
// exiled blue card) casts Force of Will with an empty mana pool.
