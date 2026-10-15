//! Plan counter results without exposing a partially committed damage event.

use super::{CounterKind, DamageTarget, DamageWork, GameState, KeywordSet, Pending};
use crate::choice::YesNoPrompt;
use crate::replacement::counter_order;
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::types::TypeSet;

impl DamageWork {
    pub(super) fn plan_counter_results(&mut self, state: &mut GameState) {
        // CR 120.4b–c: damage replacements are complete before its results
        // are processed. Read every source before any result changes it.
        state.refresh_characteristics();
        let mut groups: Vec<(ObjectId, PlayerId, bool, u16)> = Vec::new();
        for part in &mut self.parts {
            if let Some(source) = state.damage_source(part.view.source, part.source_version) {
                part.keywords = source.characteristics().keywords;
                part.controller = source.controller;
            }
            if !part.recipient_exists(state) {
                part.view.amount = 0;
            }
            let DamageTarget::Object(target) = part.view.recipient else {
                continue;
            };
            if part.view.amount == 0
                || !part.keywords.contains(KeywordSet::INFECT)
                || !state.object(target).is_some_and(|object| {
                    object.characteristics().types.contains(TypeSet::CREATURE)
                })
            {
                continue;
            }
            let amount = u16::try_from(part.view.amount).unwrap_or(u16::MAX);
            let by_effect = !part.view.is_combat;
            if let Some((_, _, _, total)) = groups.iter_mut().find(|(id, by, effect, _)| {
                *id == target && *by == part.controller && *effect == by_effect
            }) {
                *total = total.saturating_add(amount);
            } else {
                groups.push((target, part.controller, by_effect, amount));
            }
        }
        self.result_plans = Some(
            groups
                .into_iter()
                .filter_map(|(target, by, by_effect, n)| {
                    counter_order::plan(
                        state,
                        by,
                        target,
                        vec![(CounterKind::M1M1, n)],
                        by_effect,
                        None,
                    )
                })
                .collect(),
        );
    }

    pub(super) fn prepare_counter_results(&mut self, state: &GameState) -> Option<Pending> {
        let plans = self.result_plans.as_mut().expect("damage results planned");
        // Concessions may remove recipients or change their controller. Keep
        // choices already applied, but recalculate the next affected player.
        plans.retain(|plan| {
            state
                .object(plan.recipient.object)
                .is_some_and(|object| object.version == plan.recipient.version)
        });
        let mut next = None;
        for (index, plan) in plans.iter_mut().enumerate() {
            let controller = state
                .object(plan.recipient.object)
                .expect("retained recipient")
                .controller;
            while let Some(additional) = plan.question() {
                if state.has_left(controller) {
                    plan.answer(true);
                    continue;
                }
                let order = Self::apnap(state, controller);
                if next.is_none_or(|(_, _, _, first)| order < first) {
                    next = Some((index, controller, additional, order));
                }
                break;
            }
        }
        let (index, player, additional, _) = next?;
        self.result_question = Some(index);
        Some(Pending::YesNo {
            player,
            prompt: YesNoPrompt::ApplyAdditionalCountersNext {
                permanent: plans[index].recipient.object,
                additional,
            },
            source: None,
        })
    }
}
