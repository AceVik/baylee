//! Triassic Egg — {4} — Artifact
//! Oracle: {3}, {T}: Put a hatchling counter on this artifact.
//! Oracle: Sacrifice this artifact: Choose one. Activate only if there are two or more hatchling counters on this artifact.
//! Oracle: • You may put a creature card from your hand onto the battlefield.
//! Oracle: • Return target creature card from your graveyard to the battlefield.
//! Set: ME4 #235 — Masters Edition IV | Scryfall ID: 7ccd4ea9-1552-4fcc-a443-0718b81dc6a6 | Oracle ID: 4f27abe8-3e98-4bb8-afdb-e4b718d83032
// PARTIAL — {3}, {T} puts a hatchling counter; the sacrifice Charm is off
// the card, see the NOT SUPPORTED line above `abilities`.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TRIASSIC_EGG,
    oracle_id = "4f27abe8-3e98-4bb8-afdb-e4b718d83032",
    scryfall_id = "7ccd4ea9-1552-4fcc-a443-0718b81dc6a6",
    faces = &[face!(
        name = "Triassic Egg",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "no modal activated ability exists, so the sacrifice Charm's \"Choose \
         one\" cannot be stated"
    ),
    // NOT SUPPORTED: "Sacrifice this artifact: Choose one. Activate only if
    // there are two or more hatchling counters on this artifact. • You may put
    // a creature card from your hand onto the battlefield. • Return target
    // creature card from your graveyard to the battlefield." — both modes are
    // sayable (`Effect::PutFromHandOntoBattlefield` with `optional: true`, and
    // `Effect::reanimate(TargetSpec::CardInGraveyard(&Filter::CREATURE,
    // PlayerRel::You))`) and so is the gate
    // (`Condition::CountersOnSelf(counters::HATCHLING, 2)`), but `AbilityDef`
    // has `ModalSpell` and `ModalTriggered` and no modal activated ability, so
    // one sacrifice ability whose mode is chosen as it is activated cannot be
    // stated; two separate sacrifice abilities would be two abilities to copy,
    // counter or activate and would not be the Charm.
    abilities = &[activated!(
        cost!("{3}", TapSelf),
        &[Effect::AddCounter {
            kind: counters::HATCHLING,
            amount: Amount::Fixed(1),
        }],
    )],
);
