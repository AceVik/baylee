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
            // No subject: the ability said "target" and got none.
            let target_id = this_object(res)?;
            crate::replacement::put_counters(state, target_id, kind, n);
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
            // Per object, not once for the sweep: "a permanent you control"
            // is asked of each permanent the counters land on, so a board
            // holding one of mine and one of theirs gets two answers.
            for id in objects {
                crate::replacement::put_counters(state, id, kind, n);
            }
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
            // The drain half above is removal and stays where it is. The
            // placement half is a resolving ability's effect putting
            // counters on a permanent, which is the first case CR 614.16
            // names, so it goes through the door that applies the
            // multiplying replacements — a Thief of Blood under a Doubling
            // Season arrives twice the size. Writing `counters.add` here
            // also skipped the journal entry, so nothing downstream could
            // see the counters land.
            if drained > 0 {
                crate::replacement::put_counters(
                    state,
                    res.source,
                    baylee_cards_dsl::CounterKind::P1P1,
                    drained,
                );
            }
            // Kept beside the door's own invalidation: the drain is a
            // characteristic change too, and it happens whether or not
            // anything is placed afterwards.
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
            // Read before anything is registered: an ability that said
            // "target" and got none sets the power and toughness of nobody.
            let this = if matches!(filter, baylee_cards_dsl::Filter::This) {
                this_object(res)?
            } else {
                res.source
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
                // `This` means the target here, exactly as it does in
                // `CreateContinuousEffect` — the two are written as one
                // sentence on a card ("becomes a creature with power and
                // toughness equal to its mana value") and had to be able to
                // name the same object. Without this arm the only way to
                // write the P/T half was a filter describing the *kind* of
                // permanent, which then set the P/T of every permanent of
                // that kind on every battlefield. Karn's +1 on an artifact
                // land is what found it: the land's mana value is nought, so
                // every noncreature artifact in the game became a 0/0 and
                // was put into a graveyard by the next state-based check.
                filter: if matches!(filter, baylee_cards_dsl::Filter::This) {
                    crate::effects::EffectFilter::ObjectIs(this)
                } else {
                    crate::effects::EffectFilter::Dsl(filter)
                },
                modifier: baylee_cards_dsl::Modifier::SetPT(p, t),
            });
            None
        }
        Effect::PumpFilter {
            filter,
            power,
            toughness,
            keywords,
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
            pump(
                state,
                res,
                you,
                crate::effects::EffectFilter::Dsl(filter),
                (p, t),
                keywords,
                duration,
            );
            None
        }
        Effect::PumpTarget {
            power,
            toughness,
            keywords,
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
            // Every target, not just the first: a spell that pumps two
            // creatures is one effect per creature, because an
            // `EffectFilter` names exactly one object.
            for target in res.targets.clone() {
                pump(
                    state,
                    res,
                    you,
                    crate::effects::EffectFilter::ObjectIs(target),
                    (p, t),
                    keywords,
                    duration,
                );
            }
            None
        }
        _ => unreachable!("not a counter/P-T effect"),
    }
}

/// Registers one pump: a P/T modifier, and a keyword grant beside it when
/// the effect carries keywords.
///
/// The two are separate `ContinuousEffect`s because they sit in different
/// layers — keywords in layer 6, P/T in 7c (CR 613.1) — and a `Modifier`
/// carries one change. They share a timestamp so nothing can order itself
/// between the halves of a single pump.
fn pump(
    state: &mut GameState,
    res: &Resolution,
    you: baylee_core::ids::PlayerId,
    filter: crate::effects::EffectFilter,
    pt: (i16, i16),
    keywords: baylee_cards_dsl::KeywordSet,
    duration: baylee_cards_dsl::Duration,
) {
    let timestamp = state.next_timestamp();
    let mut fx = crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: Some(res.source),
        controller: you,
        layer: baylee_cards_dsl::Layer::PtModify,
        timestamp,
        duration,
        filter,
        modifier: baylee_cards_dsl::Modifier::ModifyPT(pt.0, pt.1),
    };
    // A pump of +0/+0 with keywords is Rush of Blood's shape, not a bug:
    // register the P/T half only when it moves something.
    if pt != (0, 0) {
        state.effects.register(fx.clone());
    }
    if !keywords.is_empty() {
        fx.layer = baylee_cards_dsl::Layer::Ability;
        fx.modifier = baylee_cards_dsl::Modifier::AddKeyword(keywords);
        state.effects.register(fx);
    }
}
