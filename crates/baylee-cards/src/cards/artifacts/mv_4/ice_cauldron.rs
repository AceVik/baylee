//! Ice Cauldron — {4} — Artifact
//! Oracle: {X}, {T}: You may exile a nonland card from your hand. You may cast that card for as long as it remains exiled. Put a charge counter on this artifact and note the type and amount of mana spent to pay this activation cost. Activate only if there are no charge counters on this artifact.
//! Oracle: {T}, Remove a charge counter from this artifact: Add this artifact's last noted type and amount of mana. Spend this mana only to cast the last card exiled with this artifact.
//! Set: ME4 #206 — Masters Edition IV | Scryfall ID: 5fad0f0f-b302-4f97-9cbb-a66dbfc57bae | Oracle ID: 2b7e4e70-53b1-4d4f-bdcc-56c2e15f367a
use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: exiling a nonland card from your hand with cast permission while exiled
// and noting/producing the type and amount of mana spent to pay the {X} activation cost.
card!(
    index = index::ICE_CAULDRON,
    oracle_id = "2b7e4e70-53b1-4d4f-bdcc-56c2e15f367a",
    scryfall_id = "5fad0f0f-b302-4f97-9cbb-a66dbfc57bae",
    coverage = Coverage::Partial(
        "exiling card from hand to cast later and noting/producing spent mana not supported"
    ),
    faces = &[face!(
        name = "Ice Cauldron",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{X}", TapSelf),
        &[Effect::AddCounter {
            kind: CounterKind::Charge,
            amount: Amount::Fixed(1),
        }],
        condition = Some(Condition::CountersOnSelfExactly(CounterKind::Charge, 0))
    )],
);
