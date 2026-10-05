//! Tawnos's Weaponry — {2} — Artifact
//! Oracle: You may choose not to untap this artifact during your untap step.
//! Oracle: {2}, {T}: Target creature gets +1/+1 for as long as this artifact remains tapped.
//! Set: ME4 #232 — Masters Edition IV | Scryfall ID: b73fe4b5-142f-4b1f-b48d-7aa65782bdb5 | Oracle ID: f07f98bb-4190-4643-aeb9-c5eaf358c97c
// PARTIAL — the untap-step choice is built; the pump is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TAWNOS_S_WEAPONRY,
    oracle_id = "f07f98bb-4190-4643-aeb9-c5eaf358c97c",
    scryfall_id = "b73fe4b5-142f-4b1f-b48d-7aa65782bdb5",
    faces = &[face!(
        name = "Tawnos's Weaponry",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "no Duration says \"for as long as this artifact remains tapped\": \
         WhileSourceOnBattlefield would keep the +1/+1 after the artifact \
         untaps"
    ),
    // NOT SUPPORTED: "{2}, {T}: Target creature gets +1/+1 for as long as this
    // artifact remains tapped." — `Effect::PumpTarget` with `Amount::Fixed(1)`
    // is the effect, but its duration is `Duration::WhileSourceTapped`, which
    // does not exist; the nearest, `WhileSourceOnBattlefield`, would leave the
    // +1/+1 on a creature after the Weaponry untaps, so the ability comes off
    // the card.
    abilities = &[static_ability!(Filter::This, Modifier::MayChooseNotToUntap)],
);
