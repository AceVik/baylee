//! Ice Cauldron — {4} — Artifact
//! Oracle: {X}, {T}: You may exile a nonland card from your hand. You may cast that card for as long as it remains exiled. Put a charge counter on this artifact and note the type and amount of mana spent to pay this activation cost. Activate only if there are no charge counters on this artifact.
//! Oracle: {T}, Remove a charge counter from this artifact: Add this artifact's last noted type and amount of mana. Spend this mana only to cast the last card exiled with this artifact.
//! Set: ME4 #206 — Masters Edition IV | Scryfall ID: 5fad0f0f-b302-4f97-9cbb-a66dbfc57bae | Oracle ID: 2b7e4e70-53b1-4d4f-bdcc-56c2e15f367a
use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: in the first ability, "You may exile a nonland card from your hand. You may
// cast that card for as long as it remains exiled." and "note the type and amount of mana spent
// to pay this activation cost" (only the {X}, {T} cost, the charge counter and the activation
// condition are played); and the whole second ability, "{T}, Remove a charge counter from this
// artifact: Add this artifact's last noted type and amount of mana. Spend this mana only to cast
// the last card exiled with this artifact." It is not written at all, so once the first ability
// has put its charge counter on, nothing removes it and the first ability is never available again.
card!(
    index = index::ICE_CAULDRON,
    oracle_id = "2b7e4e70-53b1-4d4f-bdcc-56c2e15f367a",
    scryfall_id = "5fad0f0f-b302-4f97-9cbb-a66dbfc57bae",
    coverage = Coverage::Partial(
        "the exile with a cast permission and the note of the mana spent are not played, and the second ability (remove a charge counter: add the noted mana) is absent"
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
