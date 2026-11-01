//! Serrated Arrows — {4} — Artifact
//! Oracle: This artifact enters with three arrowhead counters on it.
//! Oracle: At the beginning of your upkeep, if there are no arrowhead counters on this artifact, sacrifice it.
//! Oracle: {T}, Remove an arrowhead counter from this artifact: Put a -1/-1 counter on target creature.
//! Set: GVL #20 — Duel Decks Anthology: Garruk vs. Liliana | Scryfall ID: 1519b17c-9003-43e0-958f-1fcd20ab2d70 | Oracle ID: 5fa4607a-6731-4599-80f3-19d6097b4b28
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SERRATED_ARROWS,
    oracle_id = "5fa4607a-6731-4599-80f3-19d6097b4b28",
    scryfall_id = "1519b17c-9003-43e0-958f-1fcd20ab2d70",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Serrated Arrows",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: counters::ARROWHEAD,
            amount: Amount::Fixed(3),
        }],
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::SacrificeSelf],
            condition = Some(Condition::CountersOnSelfExactly(counters::ARROWHEAD, 0)),
        ),
        activated!(
            cost!(
                TapSelf,
                RemoveCounterSelf {
                    kind: counters::ARROWHEAD,
                    n: 1,
                }
            ),
            &[Effect::AddCounter {
                kind: CounterKind::M1M1,
                amount: Amount::Fixed(1),
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
    ],
);
