//! Lotus Blossom — {2} — Artifact
//! Oracle: At the beginning of your upkeep, you may put a petal counter on this artifact.
//! Oracle: {T}, Sacrifice this artifact: Add X mana of any one color, where X is the number of petal counters on this artifact.
//! Set: USG #300 — Urza's Saga | Scryfall ID: 46d17e0a-5407-4fde-8eb0-8f580eab5565 | Oracle ID: 32e66ea0-5f35-4df8-8d00-580403c51bb5
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LOTUS_BLOSSOM,
    oracle_id = "32e66ea0-5f35-4df8-8d00-580403c51bb5",
    scryfall_id = "46d17e0a-5407-4fde-8eb0-8f580eab5565",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Lotus Blossom",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::MayDo {
                effects: &[Effect::AddCounter {
                    kind: counters::PETAL,
                    amount: Amount::Fixed(1),
                }],
            }],
        ),
        mana_ability!(
            cost!(TapSelf, SacrificeSelf),
            &[Effect::mana_choice_dynamic(
                ALL_MANA_COLORS,
                Amount::CountersOnSource(counters::PETAL),
            )],
        ),
    ],
);
