//! Counters and power/toughness effects: counter placement (with
//! Doubling-Season replacement), counter drains, pump/set P/T effects.

#[allow(clippy::wildcard_imports)] // family modules share the resolve vocabulary
use super::*;

/// Executes one counter/P-T effect.
#[allow(clippy::too_many_lines)] // the family is one flat table
pub(super) fn exec(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    let you = res.controller;
    match op {
        Effect::AddCounter { kind, amount } => {
            let n = amount2(&amount, state, you, res.source, res.x, &res.targets) as u16;
            let target_id = res.targets.first().copied().unwrap_or(res.source);
            // Counter-placement replacements (Doubling Season, CR 614.2).
            let mut n_total = n;
            if let Some(target_obj) = state.object(target_id) {
                for entry in &state.replacement_rules {
                    if let baylee_cards_dsl::ReplacementRule::DoubleCounterPlacement {
                        object_filter,
                    } = entry.rule
                        && eval::matches(
                            object_filter,
                            state,
                            target_obj,
                            entry.controller,
                            entry.source,
                        )
                    {
                        n_total = n_total.saturating_mul(2);
                    }
                }
            }
            if let Some(obj) = state.object_mut(target_id) {
                let old = obj.counters.get(kind);
                let new = obj.counters.add(kind, n_total);
                state.journal.record(GameEvent::CounterChanged {
                    object: target_id,
                    kind,
                    old,
                    new,
                });
            }
            state.invalidate_projections();
            None
        }
        Effect::AddCounterFilter {
            filter,
            kind,
            amount,
        } => {
            let n = amount2(&amount, state, you, res.source, res.x, &res.targets) as u16;
            let objects: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Battlefield)
                .iter()
                .filter(|id| {
                    state
                        .object(**id)
                        .is_some_and(|o| eval::matches(filter, state, o, you, res.source))
                })
                .copied()
                .collect();
            for id in objects {
                if let Some(obj) = state.object_mut(id) {
                    let old = obj.counters.get(kind);
                    let new = obj.counters.add(kind, n);
                    state.journal.record(GameEvent::CounterChanged {
                        object: id,
                        kind,
                        old,
                        new,
                    });
                }
            }
            state.invalidate_projections();
            None
        }
        Effect::DrainAllCountersIntoSelf => {
            let mut drained: u16 = 0;
            for id in state.zones.list(ZoneLocation::Battlefield).clone() {
                if let Some(obj) = state.object_mut(id) {
                    let held: Vec<_> = obj.counters.iter().collect();
                    for (kind, n) in held {
                        if n > 0 {
                            obj.counters.set(kind, 0);
                            drained = drained.saturating_add(n);
                        }
                    }
                }
            }
            if drained > 0
                && let Some(src) = state.object_mut(res.source)
            {
                src.counters
                    .add(baylee_cards_dsl::CounterKind::P1P1, drained);
            }
            state.invalidate_projections();
            None
        }
        Effect::SetPTFilter {
            filter,
            power,
            toughness,
            duration,
        } => {
            let signed = |a: &Amount| -> i16 {
                let v = amount2(a, state, you, res.source, res.x, &res.targets) as i16;
                if matches!(a, Amount::NegX | Amount::NegXFixed(_)) {
                    -v
                } else {
                    v
                }
            };
            let p = signed(&power);
            let t = signed(&toughness);
            let ts = state.next_timestamp();
            state.effects.register(crate::effects::ContinuousEffect {
                id: baylee_core::ids::EffectId::new(0),
                source: Some(res.source),
                controller: you,
                layer: baylee_cards_dsl::Layer::PtSet,
                timestamp: ts,
                duration,
                filter: crate::effects::EffectFilter::Dsl(filter),
                modifier: baylee_cards_dsl::Modifier::SetPT(p, t),
            });
            None
        }
        Effect::PumpFilter {
            filter,
            power,
            toughness,
            duration,
        } => {
            let signed = |a: &Amount| -> i16 {
                let v = amount2(a, state, you, res.source, res.x, &res.targets) as i16;
                if matches!(a, Amount::NegX | Amount::NegXFixed(_)) {
                    -v
                } else {
                    v
                }
            };
            let p = signed(&power);
            let t = signed(&toughness);
            let ts = state.next_timestamp();
            state.effects.register(crate::effects::ContinuousEffect {
                id: baylee_core::ids::EffectId::new(0),
                source: Some(res.source),
                controller: you,
                layer: baylee_cards_dsl::Layer::PtModify,
                timestamp: ts,
                duration,
                filter: crate::effects::EffectFilter::Dsl(filter),
                modifier: baylee_cards_dsl::Modifier::ModifyPT(p, t),
            });
            None
        }
        _ => unreachable!("not a counter/P-T effect"),
    }
}
