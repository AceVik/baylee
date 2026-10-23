//! Barrin's Codex — {4} — Artifact — Book
//! Oracle: At the beginning of your upkeep, you may put a page counter on this artifact.
//! Oracle: {4}, {T}, Sacrifice this artifact: Draw X cards, where X is the number of page counters on this artifact.
//! Set: USG #286 — Urza's Saga | Scryfall ID: 9951b225-f11e-41bd-ab6e-e1fda957dff3 | Oracle ID: f7c237e6-17a4-4e72-8cc3-456aec738394
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BARRIN_S_CODEX,
    oracle_id = "f7c237e6-17a4-4e72-8cc3-456aec738394",
    scryfall_id = "9951b225-f11e-41bd-ab6e-e1fda957dff3",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Barrin's Codex",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::BOOK],
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::MayDo {
                effects: &[Effect::AddCounter {
                    kind: counters::PAGE,
                    amount: Amount::Fixed(1),
                }],
            }],
        ),
        activated!(
            cost!("{4}", TapSelf, SacrificeSelf),
            &[Effect::DrawCards {
                amount: Amount::CountersOnSource(counters::PAGE),
            }],
        ),
    ],
);
