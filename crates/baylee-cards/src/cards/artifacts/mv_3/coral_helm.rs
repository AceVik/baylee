//! Coral Helm — {3} — Artifact
//! Oracle: {3}, Discard a card at random: Target creature gets +2/+2 until end of turn.
//! Set: ME4 #194 — Masters Edition IV | Scryfall ID: 4ac5a963-d3f3-4700-8c10-6c6974b20c24 | Oracle ID: aa2970c8-f2ea-4e06-8b8f-ec89af0012a0
// PARTIAL — the pump ability is off the card: its cost is a random discard and no `CostPart` discards at random.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CORAL_HELM,
    oracle_id = "aa2970c8-f2ea-4e06-8b8f-ec89af0012a0",
    scryfall_id = "4ac5a963-d3f3-4700-8c10-6c6974b20c24",
    faces = &[face!(
        name = "Coral Helm",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "the activation cost's non-mana half is \"Discard a card at random\" and \
         no CostPart discards at random: CostPart::Discard(&Filter::Any) is a \
         discard the payer chooses, which is strictly better than the card"
    ),
    // NOT SUPPORTED: "{3}, Discard a card at random: Target creature gets
    // +2/+2 until end of turn." — `Effect::PumpTarget` with `Amount::Fixed(2)`
    // is the whole effect, but the price has no vocabulary:
    // `CostPart::Discard` asks the payer which card, and the card prints *at
    // random*, while `Effect::DiscardRandom` is an effect that would let the
    // ability resolve — and pump — after being activated with an empty hand.
    // So the ability comes off the card.
    abilities = &[],
);
