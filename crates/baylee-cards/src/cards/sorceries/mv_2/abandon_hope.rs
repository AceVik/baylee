//! Abandon Hope — {X}{1}{B} — Sorcery
//! Oracle: As an additional cost to cast this spell, discard X cards.
//! Oracle: Look at target opponent's hand and choose X cards from it. That player discards those cards.
//! Set: TMP #107 — Tempest | Scryfall ID: 942cf220-472c-48f6-8f60-993939ea5ab8 | Oracle ID: 8adbba6e-03ef-4278-aec5-8a4496b377a8
// PARTIAL — Discards a card as an additional cost to cast and lets you look at target opponent's hand and choose a card from it for that player to discard; scaling the additional discard cost and the number of chosen cards from the opponent's hand by the announced X is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ABANDON_HOPE,
    oracle_id = "8adbba6e-03ef-4278-aec5-8a4496b377a8",
    scryfall_id = "942cf220-472c-48f6-8f60-993939ea5ab8",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Discards a card as an additional cost to cast and lets you look at target opponent's hand and choose a card from it for that player to discard; scaling the additional discard cost and the number of chosen cards from the opponent's hand by the announced X is not yet modeled."
    ),
    faces = &[face!(
        name = "Abandon Hope",
        mana_cost = mana!("{X}{1}{B}"),
        types = TypeSet::SORCERY,
        mandatory_additional_costs = &[CostPart::Discard(&Filter::Any)],
    ),],
    abilities = &[spell!(
        &[Effect::RevealHandDiscard {
            filter: &Filter::Any,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )],
);
