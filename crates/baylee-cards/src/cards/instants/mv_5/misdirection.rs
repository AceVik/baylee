//! Misdirection — {3}{U}{U} — Instant
//! Oracle: You may exile a blue card from your hand rather than pay this spell's mana cost.
//! Oracle: Change the target of target spell with a single target.
//! Set: DDT #15 — Duel Decks: Merfolk vs. Goblins | Scryfall ID: c96763d6-0cea-40ed-afb2-886bfebe50a0 | Oracle ID: c39e5fb0-6de3-4105-ad3c-0ecb8951a1d5
// IMPLEMENTED — pitch cast + target redirection.

static BLUE_CARD: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Blue]));

use baylee_cards_dsl::prelude::*;

card! {
    index: 96,
    oracle_id: "c39e5fb0-6de3-4105-ad3c-0ecb8951a1d5",
    scryfall_id: "c96763d6-0cea-40ed-afb2-886bfebe50a0",
    faces: &[face! {
        name: "Misdirection",
        mana_cost: baylee_core::mana!("{3}{U}{U}"),
        types: TypeSet::INSTANT,
        alternative_costs: &[AlternativeCost {
            cost: Cost {
                mana: ManaCost::ZERO,
                parts: &[CostPart::ExileFromHand(&BLUE_CARD)],
            },
            condition: AltCondition::Always,
        }],
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[Effect::RedirectTarget {
            new_filter: &Filter::Any,
        }], targets: Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))))],
}
