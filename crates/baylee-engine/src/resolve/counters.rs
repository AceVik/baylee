//! Counters and power/toughness effects: counter placement (with
//! Doubling-Season replacement), counter drains, pump/set P/T effects.

#[allow(clippy::wildcard_imports)] // family modules share the resolve vocabulary
use super::*;

/// An amount as a **signed** P/T modifier.
///
/// The one place a pump's direction is decided, because it was three: the
/// `SetPTFilter`, `PumpFilter` and `PumpTarget` arms below each carried a
/// closure of its own, and each spelled the sign as
/// `matches!(a, Amount::NegX | Amount::NegXFixed(_))` — a positive list over
/// an enum, three times over. `Amount::is_negative` is the one answer to that
/// question and this is the one caller of it here, so a negative amount added
/// to the DSL cannot be read as a bonus by two arms out of three.
///
/// The magnitude is `amount2`'s and is unsigned on purpose. The engine
/// consumes an evaluated amount at eighteen sites and asks the sign at
/// exactly one — this one; the other seventeen are counting cards to draw,
/// tokens to make or life to gain, where a negative number has no meaning to
/// give.
fn signed(a: &Amount, state: &GameState, you: PlayerId, res: &Resolution) -> i16 {
    if matches!(a, Amount::TargetPower) {
        return target_chars(res, state).and_then(|c| c.power).unwrap_or(0);
    }
    if matches!(a, Amount::TargetToughness) {
        return target_chars(res, state)
            .and_then(|c| c.toughness)
            .unwrap_or(0);
    }
    let v = i16::try_from(amount2(a, state, you, res)).unwrap_or(i16::MAX);
    if a.is_negative() { -v } else { v }
}

pub(super) fn modify_source(
    state: &mut GameState,
    res: &Resolution,
    modifier: baylee_cards_dsl::Modifier,
    duration: baylee_cards_dsl::Duration,
) {
    let Some(source) = subjects::source(state, res).filter(|r| subjects::on_battlefield(state, *r))
    else {
        return;
    };
    let timestamp = state.next_timestamp();
    let filter = crate::effects::EffectFilter::object(state, source.object);
    if !state.source_duration_holds(duration, res.source, res.controller, filter) {
        return;
    }
    state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: Some(res.source),
        controller: res.controller,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: modifier.layer(),
        timestamp,
        duration,
        filter,
        modifier,
    });
    if matches!(modifier, baylee_cards_dsl::Modifier::GrantStatic { .. }) {
        state.refresh_characteristics();
        crate::effects::sync_granted_statics(state);
    }
}

/// Executes one counter/P-T effect.
#[allow(clippy::too_many_lines)] // the family is one flat table
pub(super) fn exec(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    let you = res.controller;
    match op {
        Effect::ChangeAttachedCounters {
            kind,
            amount,
            remove,
        } => {
            let host = eval::attached_for_ability(
                state,
                res.source,
                source_version(state, res),
                source_attachment_lki(state, res.on_stack),
            )?;
            let reference = baylee_core::ids::DamageSourceRef {
                object: host.id,
                version: host.version,
            };
            if subjects::on_battlefield(state, reference) {
                let amount = u16::try_from(amount2(&amount, state, you, res)).unwrap_or(u16::MAX);
                if remove {
                    crate::replacement::remove_counters_by(
                        state,
                        reference.object,
                        kind,
                        amount,
                        Some(res.controller),
                    );
                } else {
                    crate::replacement::put_counters_by(
                        state,
                        res.controller,
                        reference.object,
                        kind,
                        amount,
                    );
                }
            }
            None
        }
        Effect::ModifyAttached { modifier, duration } => {
            let host = eval::attached_for_ability(
                state,
                res.source,
                source_version(state, res),
                source_attachment_lki(state, res.on_stack),
            )?;
            let reference = baylee_core::ids::DamageSourceRef {
                object: host.id,
                version: host.version,
            };
            if subjects::on_battlefield(state, reference) {
                let filter =
                    crate::effects::EffectFilter::ObjectIs(reference.object, reference.version);
                if state.source_duration_holds(duration, res.source, you, filter) {
                    let timestamp = state.next_timestamp();
                    state.effects.register(crate::effects::ContinuousEffect {
                        id: baylee_core::ids::EffectId::new(0),
                        source: Some(res.source),
                        controller: you,
                        origin: crate::effects::EffectOrigin::Resolution,
                        layer: modifier.layer(),
                        timestamp,
                        duration,
                        filter,
                        modifier,
                    });
                }
            }
            None
        }
        Effect::SetSourcePTFromTarget { duration } => {
            if let Some(chars) = target_chars(res, state) {
                let modifier = baylee_cards_dsl::Modifier::SetPT(
                    chars.power.unwrap_or(0),
                    chars.toughness.unwrap_or(0),
                );
                modify_source(state, res, modifier, duration);
            }
            None
        }
        Effect::ModifySource { modifier, duration } => {
            modify_source(state, res, modifier, duration);
            None
        }
        Effect::SetSourceToughness {
            base,
            offset,
            duration,
        } => {
            let value = signed(&base, state, you, res).saturating_add(offset);
            modify_source(
                state,
                res,
                baylee_cards_dsl::Modifier::SetToughness(value),
                duration,
            );
            None
        }
        Effect::MoveCounter { from, to, kind } => {
            super::counter_move::start(state, res, from, to, kind)
        }
        Effect::MoveCounterToEvent { kind } => {
            let source = subjects::source(state, res)?;
            let target = baylee_core::ids::DamageSourceRef {
                object: res.event_object?,
                version: event_object_identity(state, res)?.0,
            };
            crate::replacement::move_counter(state, res.controller, source, target, kind);
            None
        }
        Effect::AddCounterSource { kind, amount } => {
            let source = subjects::source(state, res)?;
            if subjects::on_battlefield(state, source) {
                let n = u16::try_from(amount2(&amount, state, you, res)).unwrap_or(u16::MAX);
                crate::replacement::put_counters_by(state, res.controller, source.object, kind, n);
            }
            None
        }
        Effect::AddCountersUpTo {
            kind,
            amount,
            maximum,
        } => {
            let target = this_to_affect(state, res)?;
            let obj = state.object(target)?;
            if obj.zone != crate::zone::Zone::Battlefield {
                return None;
            }
            let room = maximum.saturating_sub(obj.counters.get(kind));
            let max = amount2(&amount, state, you, res).min(u32::from(room));
            if max == 0 {
                return None;
            }
            res.awaiting = Some(AwaitingOp::Counters {
                target,
                version: obj.version,
                kind,
                maximum,
            });
            Some(Pending::ChooseNumber {
                player: you,
                min: 0,
                max,
                reason: crate::choice::NumberPrompt::Counters { target, kind },
            })
        }
        Effect::AddCounter { kind, amount } => {
            let n = u16::try_from(amount2(&amount, state, you, res)).unwrap_or(u16::MAX);
            // "Put a +1/+1 counter on **each of** up to two target
            // creatures": every target, not the first. `this_object` answers
            // `res.targets.first()`, which is the whole truth for a
            // one-subject effect and silently dropped the second of
            // Rishkar, Peema Renegade's two — the only card in the pool that
            // names more than one. `Effect::UntapTarget` has walked
            // `res.targets` since it was written, so this is a sibling
            // reader that asked fewer questions rather than a rule nobody
            // had.
            //
            // The untargeted spelling still falls back to the source, which
            // is what "put a counter on this creature" needs; and a targeted
            // ability whose targets all became illegal resolves with an
            // empty list and puts nothing anywhere, which is the same
            // no-subject answer `this_object` gave by returning `None`.
            // A player-only target does not redirect an instruction about
            // the source (Jinxed Choker); an empty optional *object* target
            // still names nothing (Karn's +1, Yawgmoth's activation), and
            // neither does an object target that became illegal beside a
            // legal player (`subjects::names_targets`, CR 608.2b).
            if subjects::names_targets(state, res) {
                let plans = res
                    .targets
                    .iter()
                    .filter_map(|&target| {
                        subjects::target(state, res, target)
                            .filter(|&reference| subjects::is_current(state, reference))?;
                        crate::replacement::counter_order::plan(
                            state,
                            res.controller,
                            target,
                            vec![(kind, n)],
                            true,
                            None,
                        )
                    })
                    .collect();
                crate::replacement::counter_order::enqueue_batch(state, plans, Vec::new());
            } else {
                let target = this_to_affect(state, res)?;
                crate::replacement::put_counters_by(state, res.controller, target, kind, n);
            }
            None
        }
        // As an effect, remove as many as possible (CR 609.3). A `MayDo`
        // cost separately checks that the complete removal is possible.
        Effect::RemoveCounterSelf { kind, n } => {
            if !subjects::source(state, res).is_some_and(|r| {
                subjects::on_battlefield(state, r)
                    || (subjects::is_current(state, r)
                        && state
                            .object(r.object)
                            .is_some_and(|o| o.zone == crate::zone::Zone::Exile))
            }) {
                return None;
            }
            crate::replacement::remove_counters_by(
                state,
                res.source,
                kind,
                n,
                Some(res.controller),
            );
            None
        }
        Effect::RemoveAllCountersSelf { kind } => {
            let source = subjects::source(state, res)?;
            if subjects::on_battlefield(state, source) {
                let n = state.object(source.object)?.counters.get(kind);
                crate::replacement::remove_counters_by(
                    state,
                    source.object,
                    kind,
                    n,
                    Some(res.controller),
                );
            }
            None
        }
        Effect::RemoveAllCountersFilter { filter, kind } => {
            let matching: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|&id| {
                    state.object(id).is_some_and(|object| {
                        crate::eval::matches_with_context(
                            filter,
                            state,
                            object,
                            res.controller,
                            res.rule_context(),
                        )
                    })
                })
                .collect();
            for id in matching {
                let Some(amount) = state.object(id).map(|object| object.counters.get(kind)) else {
                    continue;
                };
                if amount > 0 {
                    crate::replacement::remove_counters_by(
                        state,
                        id,
                        kind,
                        amount,
                        Some(res.controller),
                    );
                }
            }
            None
        }
        Effect::RemoveAllCountersTarget => {
            for &target in &res.targets.clone() {
                if !subjects::target(state, res, target)
                    .is_some_and(|r| subjects::is_current(state, r))
                {
                    continue;
                }
                let Some(counters) = state
                    .object(target)
                    .map(|o| o.counters.iter().collect::<Vec<_>>())
                else {
                    continue;
                };
                for (kind, n) in counters {
                    crate::replacement::remove_counters_by(
                        state,
                        target,
                        kind,
                        n,
                        Some(res.controller),
                    );
                }
            }
            None
        }
        Effect::AddCounterFilter {
            filter,
            kind,
            amount,
        } => {
            let n = u16::try_from(amount2(&amount, state, you, res)).unwrap_or(u16::MAX);
            let objects: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        eval::matches_with_context(filter, state, o, you, res.rule_context())
                    })
                })
                .collect();
            // Per object, not once for the sweep: "a permanent you control"
            // is asked of each permanent the counters land on, so a board
            // holding one of mine and one of theirs gets two answers.
            let plans = objects
                .into_iter()
                .filter_map(|id| {
                    crate::replacement::counter_order::plan(
                        state,
                        res.controller,
                        id,
                        vec![(kind, n)],
                        true,
                        None,
                    )
                })
                .collect();
            crate::replacement::counter_order::enqueue_batch(state, plans, Vec::new());
            None
        }
        Effect::DoubleCountersFilter { filter, kind } => {
            // Each count is read before any counter lands, and each
            // permanent gets as many as it had (CR 701.10e) — a doubler in
            // play doubles what arrives, not what was there. A phased-out
            // creature is treated as though it doesn't exist (CR 702.26b).
            let objects: Vec<(ObjectId, u16)> = state
                .battlefield_seen()
                .filter_map(|id| {
                    let o = state.object(id)?;
                    eval::matches_with_context(filter, state, o, you, res.rule_context())
                        .then(|| (id, o.counters.get(kind)))
                })
                .filter(|(_, n)| *n > 0)
                .collect();
            let plans = objects
                .into_iter()
                .filter_map(|(id, n)| {
                    crate::replacement::counter_order::plan(
                        state,
                        res.controller,
                        id,
                        vec![(kind, n)],
                        true,
                        None,
                    )
                })
                .collect();
            crate::replacement::counter_order::enqueue_batch(state, plans, Vec::new());
            None
        }
        Effect::DrainAllCountersIntoSelf => {
            let mut drained: u16 = 0;
            for id in state.battlefield_view() {
                let held: Vec<_> = state
                    .object(id)
                    .map(|o| o.counters.iter().collect())
                    .unwrap_or_default();
                for (kind, n) in held {
                    drained = drained.saturating_add(crate::replacement::remove_counters_by(
                        state,
                        id,
                        kind,
                        n,
                        Some(res.controller),
                    ));
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
                crate::replacement::put_counters_by(
                    state,
                    res.controller,
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
            // Read before anything is registered: an ability that said
            // "target" and got none, or whose object is gone, sets the power
            // and toughness of nobody.
            let this = if matches!(filter, baylee_cards_dsl::Filter::This) {
                this_to_affect(state, res)?
            } else {
                res.source
            };
            let p = signed(&power, state, you, res);
            let t = signed(&toughness, state, you, res);
            let ts = state.next_timestamp();
            let modifier = baylee_cards_dsl::Modifier::SetPT(p, t);
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
            let filters = if matches!(filter, baylee_cards_dsl::Filter::This) {
                smallvec::smallvec![crate::effects::EffectFilter::object(state, this)]
            } else {
                super::bound_now_with_context(
                    state,
                    filter,
                    &modifier,
                    you,
                    res.rule_context(),
                    None,
                )
            };
            for filter in filters {
                state.effects.register(crate::effects::ContinuousEffect {
                    id: baylee_core::ids::EffectId::new(0),
                    source: Some(res.source),
                    controller: you,
                    origin: crate::effects::EffectOrigin::Resolution,
                    layer: baylee_cards_dsl::Layer::PtSet,
                    timestamp: ts,
                    duration,
                    filter,
                    modifier,
                });
            }
            None
        }
        Effect::ModifyFilter {
            filter,
            controlled_by,
            modifier,
            duration,
        } => {
            let seats = controlled_by.map(|rel| super::players_of(rel, state, you, res));
            let filters = if matches!(filter, baylee_cards_dsl::Filter::This) {
                let this = this_to_affect(state, res)?;
                if seats.as_ref().is_some_and(|seats| {
                    state
                        .object(this)
                        .is_none_or(|object| !seats.contains(&object.controller))
                }) {
                    return None;
                }
                smallvec::smallvec![crate::effects::EffectFilter::object(state, this)]
            } else {
                super::bound_now_with_context(
                    state,
                    filter,
                    &modifier,
                    you,
                    res.rule_context(),
                    seats.as_deref(),
                )
            };
            let timestamp = state.next_timestamp();
            for filter in filters {
                state.effects.register(crate::effects::ContinuousEffect {
                    id: baylee_core::ids::EffectId::new(0),
                    source: Some(res.source),
                    controller: you,
                    origin: crate::effects::EffectOrigin::Resolution,
                    layer: modifier.layer(),
                    timestamp,
                    duration,
                    filter,
                    modifier,
                });
            }
            None
        }
        Effect::PumpFilter {
            filter,
            controlled_by,
            power,
            toughness,
            keywords,
            duration,
        } => {
            let p = signed(&power, state, you, res);
            let t = signed(&toughness, state, you, res);
            // "Creatures target player controls get -2/-2": the seat is
            // read here, where the resolution knows what it chose, and the
            // set is the one it names on the battlefield now — the same
            // moment CR 611.2c fixes it at. A relation naming nobody (a
            // trigger that lost its target) shrinks nothing, which is the
            // whole sentence doing nothing rather than half of it.
            let seats = controlled_by.map(|rel| super::players_of(rel, state, you, res));
            let filters = if matches!(filter, baylee_cards_dsl::Filter::This) {
                let this = this_to_affect(state, res)?;
                if seats.as_ref().is_some_and(|seats| {
                    state
                        .object(this)
                        .is_none_or(|o| !seats.contains(&o.controller))
                }) {
                    return None;
                }
                smallvec::smallvec![crate::effects::EffectFilter::object(state, this)]
            } else {
                super::bound_now_with_context(
                    state,
                    filter,
                    &baylee_cards_dsl::Modifier::ModifyPT(p, t),
                    you,
                    res.rule_context(),
                    seats.as_deref(),
                )
            };
            pump(state, res, you, &filters, (p, t), keywords, duration);
            None
        }
        Effect::PumpFilterPerKeyword {
            filter,
            keywords,
            count_protection,
            duration,
        } => {
            // Bind the set before calculating any bonus (CR 611.2c). Each
            // member keeps its own count, so one effect cannot use a shared
            // `Amount` the way an ordinary PumpFilter does.
            let bound = super::bound_now_with_context(
                state,
                filter,
                &baylee_cards_dsl::Modifier::ModifyPT(1, 1),
                you,
                res.rule_context(),
                None,
            );
            for filter in bound {
                let crate::effects::EffectFilter::ObjectIs(id, version) = filter else {
                    continue;
                };
                let Some(object) = state.object(id) else {
                    continue;
                };
                let n = listed_ability_count(state, object, keywords, count_protection);
                if n > 0 {
                    pump(
                        state,
                        res,
                        you,
                        &[crate::effects::EffectFilter::ObjectIs(id, version)],
                        (n, n),
                        baylee_cards_dsl::KeywordSet::EMPTY,
                        duration,
                    );
                }
            }
            None
        }
        Effect::PumpTarget {
            power,
            toughness,
            keywords,
            duration,
        } => {
            let p = signed(&power, state, you, res);
            let t = signed(&toughness, state, you, res);
            // Every target, not just the first: a spell that pumps two
            // creatures is one effect per creature, because an
            // `EffectFilter` names exactly one object.
            let filters: SmallVec<[crate::effects::EffectFilter; 4]> = res
                .targets
                .iter()
                .map(|target| crate::effects::EffectFilter::object(state, *target))
                .collect();
            pump(state, res, you, &filters, (p, t), keywords, duration);
            None
        }
        Effect::PumpTargetSlot {
            slot,
            power,
            toughness,
            keywords,
            duration,
        } => {
            let p = signed(&power, state, you, res);
            let t = signed(&toughness, state, you, res);
            let filters = pump_slot_filters(state, res, slot);
            pump(state, res, you, &filters, (p, t), keywords, duration);
            None
        }
        _ => unreachable!("not a counter/P-T effect"),
    }
}

/// A target instance never slides into the other one's place after pruning.
fn pump_slot_filters(
    state: &GameState,
    res: &Resolution,
    slot: baylee_cards_dsl::TargetSlot,
) -> SmallVec<[crate::effects::EffectFilter; 4]> {
    use baylee_cards_dsl::TargetSlot;
    let bind = |reference: baylee_core::ids::DamageSourceRef| {
        crate::effects::EffectFilter::ObjectIs(reference.object, reference.version)
    };
    if slot == TargetSlot::This {
        return super::subjects::source(state, res)
            .filter(|reference| super::subjects::on_battlefield(state, *reference))
            .map(bind)
            .into_iter()
            .collect();
    }
    let targets = match slot {
        TargetSlot::First => res.targets.as_slice(),
        TargetSlot::Second => res.second_targets.as_slice(),
        TargetSlot::This => unreachable!("handled source"),
    };
    targets
        .iter()
        .filter_map(|target| super::subjects::target(state, res, *target))
        .filter(|reference| super::subjects::on_battlefield(state, *reference))
        .map(bind)
        .collect()
}

/// Counts distinct listed abilities on one projected object. Protection is
/// deliberately a boolean: "protection from black" and "protection from red"
/// are two variants of the one ability a card such as Akroma lists. The old
/// protection keyword bit and parameterized protection modifiers therefore
/// share that single contribution rather than being added independently.
fn listed_ability_count(
    state: &GameState,
    object: &crate::object::GameObject,
    listed: baylee_cards_dsl::KeywordSet,
    count_protection: bool,
) -> i16 {
    let projected = object.characteristics().keywords;
    let protection_bit = baylee_cards_dsl::KeywordSet::PROTECTION_BLACK;
    let mut counted = projected.bits() & listed.bits();
    if count_protection {
        counted &= !protection_bit.bits();
    }
    let has_protection = count_protection
        && (projected.contains(protection_bit)
            || state.effects.iter().any(|effect| {
                matches!(
                    effect.modifier,
                    baylee_cards_dsl::Modifier::ProtectionFrom(_)
                        | baylee_cards_dsl::Modifier::ProtectionFromExiledWithSourceTypes
                ) && crate::effects::applies_to(state, effect, object)
            }));
    i16::try_from(counted.count_ones() + u32::from(has_protection)).unwrap_or(i16::MAX)
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
    filters: &[crate::effects::EffectFilter],
    pt: (i16, i16),
    keywords: baylee_cards_dsl::KeywordSet,
    duration: baylee_cards_dsl::Duration,
) {
    // CR 611.2b: "for as long as this remains tapped" that is already over
    // as the pump would begin does nothing.
    if matches!(
        duration,
        baylee_cards_dsl::Duration::WhileSourceTapped { .. }
    ) && !source_still_tapped(state, res)
    {
        return;
    }
    let timestamp = state.next_timestamp();
    for filter in filters {
        let mut fx = crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: Some(res.source),
            controller: you,
            origin: crate::effects::EffectOrigin::Resolution,
            layer: baylee_cards_dsl::Layer::PtModify,
            timestamp,
            duration,
            filter: *filter,
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
}

#[cfg(test)]
mod pump_tests {
    #[allow(clippy::wildcard_imports)] // the family vocabulary, as above
    use super::*;
    use crate::engine::synthetic::{SyntheticLookup, preset};
    use baylee_cards_dsl::{Duration, Filter, KeywordSet, Layer, Modifier, PlayerRel};

    fn me() -> PlayerId {
        PlayerId::new(0)
    }

    fn state() -> GameState {
        GameState::from_preset(&preset(11, &[]), &SyntheticLookup::new(vec![]))
            .expect("a two-seat game")
    }

    /// A permanent with a controller and nothing else: what a pump reads is
    /// the battlefield list and the controller, and a printed card would put
    /// four other characteristics in front of the question.
    fn permanent(state: &mut GameState, seat: PlayerId, name: &str) -> ObjectId {
        let name = state.names.intern(name);
        state.create_bare(seat, ObjectKind::Permanent, name, ZoneLocation::Battlefield)
    }

    /// A resolution of `source`, controlled by seat 0, pointing at
    /// `targets`.
    fn resolution(source: ObjectId, targets: &[ObjectId]) -> Resolution {
        Resolution {
            source,
            on_stack: source,
            controller: me(),
            effects: vec![],
            pc: 0,
            targets: targets.iter().copied().collect(),
            second_targets: SmallVec::new(),
            x: None,
            chosen_player: None,
            target_players: baylee_core::ids::SeatSet::new(),
            event_object: None,
            awaiting: None,
            targeted: !targets.is_empty(),
            mana_ability: false,
            countered_source: None,
            target_lki: None,
            subject: crate::resolve::SubjectContext::default(),
            text: crate::text_changes::TextChangeMap::IDENTITY,
            event_mana: None,
            retarget_left: None,
            loot: None,
            draw_watch: None,
            this_way_from: None,
            coin_stake: None,
        }
    }

    /// `(layer, modifier, timestamp)` of everything registered, in order.
    fn registered(state: &GameState) -> Vec<(Layer, Modifier, u64)> {
        state
            .effects
            .iter()
            .map(|fx| (fx.layer, fx.modifier, fx.timestamp))
            .collect()
    }

    fn pump_target(power: Amount, toughness: Amount, keywords: KeywordSet) -> Effect {
        Effect::PumpTarget {
            power,
            toughness,
            keywords,
            duration: Duration::UntilEndOfTurn,
        }
    }

    #[test]
    fn explicit_pump_slots_do_not_slide_when_either_target_instance_is_illegal() {
        use baylee_cards_dsl::TargetSlot;
        for absent in [None, Some(TargetSlot::First), Some(TargetSlot::Second)] {
            let mut state = state();
            let first = permanent(&mut state, me(), "first target");
            let second = permanent(&mut state, me(), "second target");
            let source = permanent(&mut state, me(), "spell");
            let mut res = resolution(source, &[first]);
            res.second_targets.push(second);
            if absent == Some(TargetSlot::First) {
                res.targets.clear();
            } else if absent == Some(TargetSlot::Second) {
                res.second_targets.clear();
            }
            for (slot, power, toughness) in [
                (TargetSlot::First, Amount::NegXFixed(2), Amount::Fixed(0)),
                (TargetSlot::Second, Amount::Fixed(0), Amount::NegXFixed(5)),
            ] {
                exec(
                    &mut state,
                    &mut res,
                    Effect::PumpTargetSlot {
                        slot,
                        power,
                        toughness,
                        keywords: KeywordSet::EMPTY,
                        duration: Duration::UntilEndOfTurn,
                    },
                );
            }
            let effects: Vec<_> = state.effects.iter().collect();
            assert_eq!(effects.len(), if absent.is_some() { 1 } else { 2 });
            for effect in effects {
                let crate::effects::EffectFilter::ObjectIs(target, _) = effect.filter else {
                    panic!("pump is bound to an exact target");
                };
                assert_eq!(
                    effect.modifier,
                    if target == first {
                        assert_ne!(absent, Some(TargetSlot::First));
                        Modifier::ModifyPT(-2, 0)
                    } else {
                        assert_eq!(target, second);
                        assert_ne!(absent, Some(TargetSlot::Second));
                        Modifier::ModifyPT(0, -5)
                    }
                );
            }
        }
    }

    /// **One printed sentence, two continuous effects, one timestamp.**
    ///
    /// "Target creature gets +2/+2 and gains flying until end of turn" is a
    /// single pump, and a `Modifier` carries one change — so it has to be
    /// registered twice. CR 613.1 puts the two halves in different layers
    /// and applies them in layer order, keywords (6) before power and
    /// toughness (7c), which is why they cannot be one effect however much
    /// the card reads like one thing.
    ///
    /// The shared timestamp is the half that is invisible until something
    /// else happens in between. Timestamps order effects *within* a layer
    /// (CR 613.7), so two calls to `next_timestamp` here would leave a gap
    /// another effect could be registered into — and the two halves of one
    /// sentence would then be ordered on either side of an effect that
    /// arrived after both of them.
    #[test]
    fn a_pump_with_keywords_is_two_effects_in_two_layers_under_one_timestamp() {
        let mut state = state();
        let bear = permanent(&mut state, me(), "Grizzly Bears");
        let source = permanent(&mut state, me(), "Giant Growth");
        let mut res = resolution(source, &[bear]);

        assert!(state.effects.is_empty(), "nothing is registered yet");
        exec(
            &mut state,
            &mut res,
            pump_target(Amount::Fixed(2), Amount::Fixed(2), KeywordSet::FLYING),
        );

        let fx = registered(&state);
        assert_eq!(fx.len(), 2, "one sentence, two effects");
        assert_eq!(fx[0].0, Layer::PtModify);
        assert_eq!(fx[0].1, Modifier::ModifyPT(2, 2));
        assert_eq!(fx[1].0, Layer::Ability);
        assert_eq!(fx[1].1, Modifier::AddKeyword(KeywordSet::FLYING));
        assert_eq!(
            fx[0].2, fx[1].2,
            "the halves of one pump share a timestamp, so nothing can be \
             ordered between them"
        );

        // And the two are about the creature that was targeted, not about
        // the spell that did the targeting.
        for effect in state.effects.iter() {
            assert!(
                matches!(effect.filter, crate::effects::EffectFilter::ObjectIs(id, _) if id == bear),
                "a pump names its target: {:?}",
                effect.filter
            );
            assert_eq!(effect.controller, me());
            assert_eq!(effect.duration, Duration::UntilEndOfTurn);
        }
    }

    /// **Each half is registered only when it moves something**, which is
    /// what makes Rush of Blood's shape — a `+0/+0` carrying keywords —
    /// something other than a defect.
    ///
    /// The P/T half of that would be a continuous effect in layer 7c adding
    /// nought to a power and nought to a toughness: it changes no
    /// characteristic and still takes a timestamp, so it is a rank in the
    /// layer's ordering that nothing printed asked for. The keyword half of
    /// a plain Giant Growth is the same nothing from the other side.
    #[test]
    fn a_pump_registers_the_halves_that_move_something_and_no_others() {
        let cases: [(Amount, KeywordSet, &[Layer]); 4] = [
            (
                Amount::Fixed(0),
                KeywordSet::FLYING,
                &[Layer::Ability], // Rush of Blood: keywords and no numbers
            ),
            (
                Amount::Fixed(2),
                KeywordSet::EMPTY,
                &[Layer::PtModify], // Giant Growth: numbers and no keywords
            ),
            (
                Amount::Fixed(2),
                KeywordSet::FLYING,
                &[Layer::PtModify, Layer::Ability],
            ),
            (Amount::Fixed(0), KeywordSet::EMPTY, &[]),
        ];

        for (amount, keywords, expected) in cases {
            let mut state = state();
            let bear = permanent(&mut state, me(), "Grizzly Bears");
            let source = permanent(&mut state, me(), "Rush of Blood");
            let mut res = resolution(source, &[bear]);
            exec(&mut state, &mut res, pump_target(amount, amount, keywords));

            let layers: Vec<Layer> = registered(&state).into_iter().map(|f| f.0).collect();
            assert_eq!(
                layers, expected,
                "{amount:?} with {keywords:?} registers the wrong halves"
            );
        }
    }

    /// **The sign is read per half.** `signed` is the one place a pump's
    /// direction is decided, and it is called once for the power and once
    /// for the toughness — a reader that took the direction from one of them
    /// and applied it to both would be right on every symmetric pump in the
    /// pool and wrong on the asymmetric ones, which is a great many cards
    /// reading `+2/-2`.
    ///
    /// The magnitude is unsigned on purpose, so `NegXFixed(2)` is a two that
    /// happens to point downwards rather than a minus two.
    #[test]
    fn each_half_of_a_pump_carries_its_own_direction() {
        for (power, toughness, expected) in [
            (Amount::Fixed(2), Amount::Fixed(2), Modifier::ModifyPT(2, 2)),
            (
                Amount::NegXFixed(2),
                Amount::NegXFixed(2),
                Modifier::ModifyPT(-2, -2),
            ),
            (
                Amount::Fixed(2),
                Amount::NegXFixed(2),
                Modifier::ModifyPT(2, -2),
            ),
            (
                Amount::NegXFixed(3),
                Amount::Fixed(1),
                Modifier::ModifyPT(-3, 1),
            ),
        ] {
            let mut state = state();
            let bear = permanent(&mut state, me(), "Grizzly Bears");
            let source = permanent(&mut state, me(), "the spell");
            let mut res = resolution(source, &[bear]);
            exec(
                &mut state,
                &mut res,
                pump_target(power, toughness, KeywordSet::EMPTY),
            );

            let fx = registered(&state);
            assert_eq!(fx.len(), 1);
            assert_eq!(fx[0].1, expected, "{power:?}/{toughness:?}");
        }
    }

    /// **Every target, not just the first.** An `EffectFilter` names exactly
    /// one object, so a spell that pumps two creatures is two effects — and
    /// a reader that registered against `targets.first()` would leave the
    /// second creature unchanged while the spell resolved and the journal
    /// said it had.
    #[test]
    fn a_pump_is_registered_once_per_target() {
        let mut state = state();
        let first = permanent(&mut state, me(), "Grizzly Bears");
        let second = permanent(&mut state, me(), "Runeclaw Bear");
        let source = permanent(&mut state, me(), "the spell");
        let mut res = resolution(source, &[first, second]);

        exec(
            &mut state,
            &mut res,
            pump_target(Amount::Fixed(1), Amount::Fixed(1), KeywordSet::EMPTY),
        );

        let named: Vec<ObjectId> = state
            .effects
            .iter()
            .map(|fx| match fx.filter {
                crate::effects::EffectFilter::ObjectIs(id, _) => id,
                crate::effects::EffectFilter::Dsl(f)
                | crate::effects::EffectFilter::DslAbout(f, ..)
                | crate::effects::EffectFilter::DslControlledBy(f, ..) => {
                    panic!("a dynamic filter: {f:?}")
                }
            })
            .collect();
        assert_eq!(named, vec![first, second]);
    }

    /// **A relation that names nobody shrinks nothing**, which is the whole
    /// sentence doing nothing rather than half of it.
    ///
    /// "Creatures target player controls get -2/-2" reads its seat where the
    /// resolution knows what it chose. A trigger that lost its target
    /// chooses no player, and the set is then empty rather than absent — the
    /// difference between shrinking nobody's creatures and shrinking
    /// everybody's, because a `None` seat list is what `PumpFilter` passes
    /// when the sentence names no player at all.
    #[test]
    fn a_pump_at_a_player_nobody_chose_touches_no_creature() {
        static CREATURES: Filter = Filter::CREATURE;

        let mut state = state();
        let mine = permanent(&mut state, me(), "Grizzly Bears");
        let theirs = permanent(&mut state, PlayerId::new(1), "Runeclaw Bear");
        for id in [mine, theirs] {
            state.object_mut(id).expect("just created").base_mut().types =
                baylee_core::types::TypeSet::CREATURE;
        }
        let source = permanent(&mut state, me(), "the trigger");
        let mut res = resolution(source, &[]);
        res.chosen_player = None;

        let shrink = Effect::PumpFilter {
            filter: &CREATURES,
            controlled_by: Some(PlayerRel::Chosen),
            power: Amount::NegXFixed(2),
            toughness: Amount::NegXFixed(2),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        };
        exec(&mut state, &mut res, shrink);
        assert!(
            state.effects.is_empty(),
            "no player was chosen, so no creature was named"
        );

        // The counter-evidence: the same sentence with a player behind it
        // reaches that player's creatures and nobody else's.
        res.chosen_player = Some(PlayerId::new(1));
        exec(&mut state, &mut res, shrink);
        let named: Vec<ObjectId> = state
            .effects
            .iter()
            .map(|fx| match fx.filter {
                crate::effects::EffectFilter::ObjectIs(id, _) => id,
                crate::effects::EffectFilter::Dsl(f)
                | crate::effects::EffectFilter::DslAbout(f, ..)
                | crate::effects::EffectFilter::DslControlledBy(f, ..) => {
                    panic!("a dynamic filter: {f:?}")
                }
            })
            .collect();
        assert_eq!(named, vec![theirs]);
    }
    #[test]
    fn bounded_counter_choice_finishes_once_and_never_removes_existing_counters() {
        use baylee_cards_dsl::CounterKind;
        for (existing, chosen, expected) in [(5, 0, 5), (5, 2, 7), (9, 0, 9)] {
            let mut state = state();
            let source = permanent(&mut state, me(), "Bounded counters");
            state
                .object_mut(source)
                .unwrap()
                .counters
                .add(CounterKind::P1P1, existing);
            let before = state.players[0].life;
            let mut res = resolution(source, &[]);
            res.effects = vec![
                Effect::AddCountersUpTo {
                    kind: CounterKind::P1P1,
                    amount: Amount::Fixed(4),
                    maximum: 7,
                },
                Effect::GainLife {
                    amount: Amount::Fixed(1),
                },
            ];
            let flow = run(&mut state, &mut res);
            if existing < 7 {
                assert!(matches!(
                    flow,
                    Flow::Wait(Pending::ChooseNumber { min: 0, max: 2, .. })
                ));
                assert!(matches!(
                    resume_with_number(&mut state, &mut res, chosen),
                    Flow::Complete
                ));
            } else {
                assert!(matches!(flow, Flow::Complete));
            }
            assert_eq!(
                state
                    .object(source)
                    .unwrap()
                    .counters
                    .get(CounterKind::P1P1),
                expected
            );
            assert_eq!(
                state.players[0].life,
                before + 1,
                "the next instruction runs exactly once"
            );
        }
    }
}
