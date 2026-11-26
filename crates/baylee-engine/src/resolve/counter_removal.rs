//! Removing counters from a targeted permanent or player.
//!
//! Complete removal is synchronous. A bounded removal asks once about each
//! kind present: any number up to the remaining maximum is legal, including
//! zero. A separate yes/no choice then continues with another kind or stops
//! the instruction. Asking once per kind keeps a sequence of zeroes finite,
//! without restricting a player's distribution over the available kinds.

#[allow(clippy::wildcard_imports)] // the resolve family's shared vocabulary
use super::*;

/// Remove every counter, or begin an up-to-`maximum` distribution.
pub(super) fn start(
    state: &mut GameState,
    res: &mut Resolution,
    maximum: Option<u16>,
) -> Option<Pending> {
    for target in res.targets.clone() {
        let Some(target) = subjects::target(state, res, target)
            .filter(|target| subjects::is_current(state, *target))
        else {
            continue;
        };
        if let Some(maximum) = maximum {
            let kinds = present(state, target.object);
            return next(state, res, target, kinds, maximum, 0);
        }
        for (kind, count) in present_with_counts(state, target.object) {
            crate::replacement::remove_counters_by(
                state,
                target.object,
                kind,
                count,
                Some(res.controller),
            );
        }
    }
    for player in res.target_players.iter() {
        if state.has_left(player) {
            continue;
        }
        if maximum.is_some() {
            // Bounded removal is a permanent-only sentence. Keep an
            // accidentally broad target from silently becoming "remove all".
            continue;
        }
        let kinds = state.players[usize::from(player.get())].counters();
        for (kind, count) in kinds.into_iter().filter(|(_, count)| *count > 0) {
            state.change_player_counters(player, kind, -i32::from(count));
        }
    }
    None
}

fn present_with_counts(
    state: &GameState,
    object: ObjectId,
) -> Vec<(baylee_cards_dsl::CounterKind, u16)> {
    state.object(object).map_or_default(|object| {
        object
            .counters
            .iter()
            .filter(|(_, count)| *count > 0)
            .collect()
    })
}

fn present(state: &GameState, object: ObjectId) -> Vec<baylee_cards_dsl::CounterKind> {
    state.object(object).map_or_default(|object| {
        object
            .counters
            .iter()
            .filter_map(|(kind, count)| (count > 0).then_some(kind))
            .collect()
    })
}

/// Ask about the next kind with a positive count still available.
fn next(
    state: &mut GameState,
    res: &mut Resolution,
    target: baylee_core::ids::DamageSourceRef,
    mut remaining: Vec<baylee_cards_dsl::CounterKind>,
    maximum: u16,
    removed: u16,
) -> Option<Pending> {
    if removed >= maximum || !subjects::on_battlefield(state, target) {
        return None;
    }
    while let Some(kind) = remaining.first().copied() {
        let available = state
            .object(target.object)
            .map_or(0, |object| object.counters.get(kind));
        if available == 0 {
            remaining.remove(0);
            continue;
        }
        let max = available.min(maximum - removed);
        res.awaiting = Some(AwaitingOp::RemoveUpToCountersTarget {
            target,
            kind,
            remaining,
            maximum,
            removed,
        });
        return Some(Pending::ChooseNumber {
            player: res.controller,
            min: 0,
            max: u32::from(max),
            reason: crate::choice::NumberPrompt::Counters {
                target: target.object,
                kind,
            },
        });
    }
    None
}

/// Apply a numeric answer and offer another kind or the end of the removal.
#[allow(clippy::too_many_arguments)] // mirrors the stored RemoveUpToCountersTarget continuation
pub(super) fn resume(
    state: &mut GameState,
    res: &mut Resolution,
    number: u32,
    target: baylee_core::ids::DamageSourceRef,
    kind: baylee_cards_dsl::CounterKind,
    mut remaining: Vec<baylee_cards_dsl::CounterKind>,
    maximum: u16,
    removed: u16,
) -> Flow {
    if !subjects::on_battlefield(state, target) {
        res.pc += 1;
        return run(state, res);
    }
    let available = state
        .object(target.object)
        .map_or(0, |object| object.counters.get(kind));
    let chosen = u16::try_from(number)
        .unwrap_or(u16::MAX)
        .min(available)
        .min(maximum.saturating_sub(removed));
    let actually_removed = crate::replacement::remove_counters_by(
        state,
        target.object,
        kind,
        chosen,
        Some(res.controller),
    );
    let removed = removed.saturating_add(actually_removed);
    remaining.retain(|remaining_kind| {
        *remaining_kind != kind
            && state
                .object(target.object)
                .is_some_and(|object| object.counters.get(*remaining_kind) > 0)
    });
    if removed < maximum && !remaining.is_empty() && subjects::on_battlefield(state, target) {
        res.awaiting = Some(AwaitingOp::ContinueRemovingCounters {
            target,
            remaining,
            maximum,
            removed,
        });
        return Flow::Wait(Pending::YesNo {
            player: res.controller,
            prompt: YesNoPrompt::ContinueRemovingCounters {
                target: target.object,
                remaining: maximum - removed,
            },
            source: resolving_ability(state, res),
        });
    }
    res.pc += 1;
    run(state, res)
}

/// Continue with the remaining kinds, or finish this instruction.
pub(super) fn resume_continue(
    state: &mut GameState,
    res: &mut Resolution,
    answer: bool,
    target: baylee_core::ids::DamageSourceRef,
    remaining: Vec<baylee_cards_dsl::CounterKind>,
    maximum: u16,
    removed: u16,
) -> Flow {
    if answer && let Some(pending) = next(state, res, target, remaining, maximum, removed) {
        return Flow::Wait(pending);
    }
    res.pc += 1;
    run(state, res)
}

/// Target permanent or player losing counters in [`Effect::RemoveCounters`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum RemoveSubjectRef {
    /// Battlefield permanent incarnation.
    Object(baylee_core::ids::DamageSourceRef),
    /// Living player seat.
    Player(PlayerId),
}

/// Evaluated quantity shape stored across a [`RemoveCounterSubject::Choice`] prompt.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum EvaluatedRemoveCount {
    /// Remove exactly `u16` counters (clamped to available).
    Exact(u16),
    /// Remove up to `u16` counters.
    UpTo(u16),
    /// Remove any number of counters.
    AnyNumber,
    /// Remove all matching counters.
    All,
}

/// Active step for [`AwaitingOp::RemoveCountersChoice`].
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum RemoveCountersStep {
    /// Non-targeted permanent choice (`Pending::ChooseCards`, `min: 1, max: 1`).
    ChoosePermanent {
        /// Optional specific counter kind, or `None` for any kind.
        kind: Option<baylee_cards_dsl::CounterKind>,
        /// Evaluated removal quantity to apply to the chosen permanent.
        count: EvaluatedRemoveCount,
    },
    /// Choosing one counter kind to remove (`Pending::ChooseCastMode`).
    PickSingleKind {
        /// Permanent or player losing the counter.
        subject: RemoveSubjectRef,
        /// Distinct removable counter kinds offered.
        kinds: Vec<baylee_cards_dsl::CounterKind>,
    },
    /// Choosing counts per `(subject, kind)` (`Pending::ChooseNumber`).
    ChooseCount {
        /// `(subject, kind)` pairs in deterministic order.
        queue: Vec<(RemoveSubjectRef, baylee_cards_dsl::CounterKind)>,
        /// Current index in `queue`.
        index: usize,
        /// Remaining budget (`Some(n)` for `Exact`/`UpTo`, `None` for `AnyNumber`).
        remaining_budget: Option<u16>,
        /// Whether `remaining_budget` must be fully spent if available.
        exact_required: bool,
    },
}

/// Resumable payload for [`AwaitingOp::RemoveCountersChoice`].
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct RemoveCountersPlan {
    /// Current interactive step.
    pub step: RemoveCountersStep,
}

fn subject_counter_count(
    state: &GameState,
    subj: RemoveSubjectRef,
    kind: baylee_cards_dsl::CounterKind,
) -> u16 {
    match subj {
        RemoveSubjectRef::Object(r) => {
            if !subjects::on_battlefield(state, r) {
                return 0;
            }
            state.object(r.object).map_or(0, |o| o.counters.get(kind))
        }
        RemoveSubjectRef::Player(p) => {
            if state.has_left(p) {
                return 0;
            }
            state.players[usize::from(p.get())]
                .counters()
                .into_iter()
                .find_map(|(k, n)| (k == kind).then_some(n))
                .unwrap_or(0)
        }
    }
}

fn subject_present_kinds(
    state: &GameState,
    subj: RemoveSubjectRef,
    filter_kind: Option<baylee_cards_dsl::CounterKind>,
) -> Vec<(baylee_cards_dsl::CounterKind, u16)> {
    match subj {
        RemoveSubjectRef::Object(r) => {
            if !subjects::on_battlefield(state, r) {
                return Vec::new();
            }
            present_with_counts(state, r.object)
                .into_iter()
                .filter(|&(k, n)| n > 0 && filter_kind.is_none_or(|w| w == k))
                .collect()
        }
        RemoveSubjectRef::Player(p) => {
            if state.has_left(p) {
                return Vec::new();
            }
            state.players[usize::from(p.get())]
                .counters()
                .into_iter()
                .filter(|&(k, n)| n > 0 && filter_kind.is_none_or(|w| w == k))
                .collect()
        }
    }
}

fn remove_from_subject(
    state: &mut GameState,
    controller: PlayerId,
    subj: RemoveSubjectRef,
    kind: baylee_cards_dsl::CounterKind,
    amount: u16,
) {
    if amount == 0 {
        return;
    }
    match subj {
        RemoveSubjectRef::Object(r) => {
            if subjects::on_battlefield(state, r) {
                crate::replacement::remove_counters_by(
                    state,
                    r.object,
                    kind,
                    amount,
                    Some(controller),
                );
            }
        }
        RemoveSubjectRef::Player(p) => {
            if !state.has_left(p) {
                state.change_player_counters(p, kind, -i32::from(amount));
            }
        }
    }
}

/// Starts executing [`baylee_cards_dsl::Effect::RemoveCounters`].
#[allow(clippy::too_many_lines)]
pub(super) fn start_generic(
    state: &mut GameState,
    res: &mut Resolution,
    subject: baylee_cards_dsl::RemoveCounterSubject,
    kind: Option<baylee_cards_dsl::CounterKind>,
    count: baylee_cards_dsl::RemoveCounterCount,
) -> Option<Pending> {
    let you = res.controller;
    let evaluated = match count {
        baylee_cards_dsl::RemoveCounterCount::Exact(a) => EvaluatedRemoveCount::Exact(
            u16::try_from(amount2(&a, state, you, res)).unwrap_or(u16::MAX),
        ),
        baylee_cards_dsl::RemoveCounterCount::UpTo(a) => EvaluatedRemoveCount::UpTo(
            u16::try_from(amount2(&a, state, you, res)).unwrap_or(u16::MAX),
        ),
        baylee_cards_dsl::RemoveCounterCount::AnyNumber => EvaluatedRemoveCount::AnyNumber,
        baylee_cards_dsl::RemoveCounterCount::All => EvaluatedRemoveCount::All,
    };

    if let baylee_cards_dsl::RemoveCounterSubject::Choice(filter) = subject {
        let options: Vec<ObjectId> = state
            .battlefield_seen()
            .filter(|&id| {
                state.object(id).is_some_and(|o| {
                    eval::matches_with_context(filter, state, o, you, res.rule_context())
                        && o.counters
                            .iter()
                            .any(|(k, n)| n > 0 && kind.is_none_or(|w| w == k))
                })
            })
            .collect();
        return match options.as_slice() {
            [] => None,
            [only] => {
                let subj = state.object(*only).map(|o| {
                    RemoveSubjectRef::Object(baylee_core::ids::DamageSourceRef {
                        object: *only,
                        version: o.version,
                    })
                })?;
                run_removal_on_subjects(state, res, vec![subj], kind, evaluated)
            }
            _ => {
                res.awaiting = Some(AwaitingOp::RemoveCountersChoice(Box::new(
                    RemoveCountersPlan {
                        step: RemoveCountersStep::ChoosePermanent {
                            kind,
                            count: evaluated,
                        },
                    },
                )));
                Some(Pending::ChooseCards {
                    player: res.controller,
                    options,
                    min: 1,
                    max: 1,
                    prompt: ChoicePrompt::Generic,
                    total: None,
                })
            }
        };
    }

    let mut targets: Vec<RemoveSubjectRef> = Vec::new();
    match subject {
        baylee_cards_dsl::RemoveCounterSubject::Targets => {
            for id in res.targets.clone() {
                if let Some(r) =
                    subjects::target(state, res, id).filter(|r| subjects::on_battlefield(state, *r))
                {
                    targets.push(RemoveSubjectRef::Object(r));
                }
            }
            for p in res.target_players.iter() {
                if !state.has_left(p) {
                    targets.push(RemoveSubjectRef::Player(p));
                }
            }
            if targets.is_empty()
                && let Some(p) = res.chosen_player
                && !state.has_left(p)
            {
                targets.push(RemoveSubjectRef::Player(p));
            }
        }
        baylee_cards_dsl::RemoveCounterSubject::Attached => {
            if let Some(host) = eval::attached_for_ability(
                state,
                res.source,
                source_version(state, res),
                source_attachment_lki(state, res.on_stack),
            ) {
                let r = baylee_core::ids::DamageSourceRef {
                    object: host.id,
                    version: host.version,
                };
                if subjects::on_battlefield(state, r) {
                    targets.push(RemoveSubjectRef::Object(r));
                }
            }
        }
        baylee_cards_dsl::RemoveCounterSubject::Source => {
            if let Some(r) =
                subjects::source(state, res).filter(|r| subjects::on_battlefield(state, *r))
            {
                targets.push(RemoveSubjectRef::Object(r));
            }
        }
        baylee_cards_dsl::RemoveCounterSubject::Player(who) => {
            for p in players_of(who, state, you, res) {
                if !state.has_left(p) {
                    targets.push(RemoveSubjectRef::Player(p));
                }
            }
        }
        baylee_cards_dsl::RemoveCounterSubject::Filter(filter) => {
            for id in state.battlefield_seen() {
                if let Some(o) = state.object(id)
                    && eval::matches_with_context(filter, state, o, you, res.rule_context())
                {
                    targets.push(RemoveSubjectRef::Object(
                        baylee_core::ids::DamageSourceRef {
                            object: id,
                            version: o.version,
                        },
                    ));
                }
            }
        }
        baylee_cards_dsl::RemoveCounterSubject::Choice(_) => unreachable!("handled above"),
    }
    run_removal_on_subjects(state, res, targets, kind, evaluated)
}

fn run_removal_on_subjects(
    state: &mut GameState,
    res: &mut Resolution,
    subjects_list: Vec<RemoveSubjectRef>,
    kind: Option<baylee_cards_dsl::CounterKind>,
    count: EvaluatedRemoveCount,
) -> Option<Pending> {
    if subjects_list.is_empty() {
        return None;
    }
    match count {
        EvaluatedRemoveCount::All => {
            for subj in subjects_list {
                for (k, n) in subject_present_kinds(state, subj, kind) {
                    remove_from_subject(state, res.controller, subj, k, n);
                }
            }
            None
        }
        EvaluatedRemoveCount::Exact(0) | EvaluatedRemoveCount::UpTo(0) => None,
        EvaluatedRemoveCount::Exact(n) => {
            let mut queue: Vec<(RemoveSubjectRef, baylee_cards_dsl::CounterKind)> = Vec::new();
            let mut total_avail: u16 = 0;
            for subj in subjects_list {
                for (k, avail) in subject_present_kinds(state, subj, kind) {
                    queue.push((subj, k));
                    total_avail = total_avail.saturating_add(avail);
                }
            }
            if queue.is_empty() {
                return None;
            }
            if total_avail <= n {
                for (subj, k) in queue {
                    let avail = subject_counter_count(state, subj, k);
                    remove_from_subject(state, res.controller, subj, k, avail);
                }
                return None;
            }
            if queue.len() == 1 {
                let (subj, k) = queue[0];
                remove_from_subject(state, res.controller, subj, k, n);
                return None;
            }
            if n == 1 && queue.iter().all(|(s, _)| *s == queue[0].0) {
                let subj = queue[0].0;
                let kinds: Vec<_> = queue.into_iter().map(|(_, k)| k).collect();
                let options = kinds
                    .iter()
                    .enumerate()
                    .map(|(index, k)| crate::choice::CastModeDesc {
                        index: u32::try_from(index).expect("finite counter-kind vocabulary"),
                        kind: crate::choice::CastModeKind::Counter(*k),
                        cost: baylee_core::mana::ManaCost::ZERO,
                    })
                    .collect();
                res.awaiting = Some(AwaitingOp::RemoveCountersChoice(Box::new(
                    RemoveCountersPlan {
                        step: RemoveCountersStep::PickSingleKind {
                            subject: subj,
                            kinds,
                        },
                    },
                )));
                return Some(Pending::ChooseCastMode {
                    player: res.controller,
                    object: res.source,
                    options,
                });
            }
            advance_remove_count(state, res, queue, 0, Some(n), true)
        }
        EvaluatedRemoveCount::UpTo(max_n) => {
            let mut queue = Vec::new();
            for subj in subjects_list {
                for (k, _) in subject_present_kinds(state, subj, kind) {
                    queue.push((subj, k));
                }
            }
            advance_remove_count(state, res, queue, 0, Some(max_n), false)
        }
        EvaluatedRemoveCount::AnyNumber => {
            let mut queue = Vec::new();
            for subj in subjects_list {
                for (k, _) in subject_present_kinds(state, subj, kind) {
                    queue.push((subj, k));
                }
            }
            advance_remove_count(state, res, queue, 0, None, false)
        }
    }
}

fn advance_remove_count(
    state: &mut GameState,
    res: &mut Resolution,
    queue: Vec<(RemoveSubjectRef, baylee_cards_dsl::CounterKind)>,
    mut index: usize,
    mut remaining_budget: Option<u16>,
    exact_required: bool,
) -> Option<Pending> {
    while index < queue.len() {
        if remaining_budget == Some(0) {
            break;
        }
        let (subj, k) = queue[index];
        let avail = subject_counter_count(state, subj, k);
        let max_here = avail.min(remaining_budget.unwrap_or(u16::MAX));
        if max_here == 0 {
            index += 1;
            continue;
        }
        let min_here = if exact_required {
            let later_avail: u16 = queue[index + 1..]
                .iter()
                .map(|&(s, ck)| subject_counter_count(state, s, ck))
                .fold(0, u16::saturating_add);
            remaining_budget
                .unwrap_or(0)
                .saturating_sub(later_avail)
                .min(max_here)
        } else {
            0
        };
        if min_here == max_here {
            remove_from_subject(state, res.controller, subj, k, min_here);
            if let Some(rem) = &mut remaining_budget {
                *rem = rem.saturating_sub(min_here);
            }
            index += 1;
            continue;
        }
        let prompt_target = match subj {
            RemoveSubjectRef::Object(r) => r.object,
            RemoveSubjectRef::Player(_) => res.source,
        };
        res.awaiting = Some(AwaitingOp::RemoveCountersChoice(Box::new(
            RemoveCountersPlan {
                step: RemoveCountersStep::ChooseCount {
                    queue,
                    index,
                    remaining_budget,
                    exact_required,
                },
            },
        )));
        return Some(Pending::ChooseNumber {
            player: res.controller,
            min: u32::from(min_here),
            max: u32::from(max_here),
            reason: crate::choice::NumberPrompt::Counters {
                target: prompt_target,
                kind: k,
            },
        });
    }
    None
}

/// Resumes [`AwaitingOp::RemoveCountersChoice`] after a `Pending::ChooseCards` answer.
pub(super) fn resume_chosen_permanent(
    state: &mut GameState,
    res: &mut Resolution,
    plan: &RemoveCountersPlan,
    chosen: &[ObjectId],
) -> Option<Pending> {
    let RemoveCountersStep::ChoosePermanent { kind, count } = plan.step else {
        unreachable!("ChooseCards only answers ChoosePermanent")
    };
    let id = *chosen.first()?;
    let obj = state.object(id)?;
    let r = baylee_core::ids::DamageSourceRef {
        object: id,
        version: obj.version,
    };
    if !subjects::on_battlefield(state, r) {
        return None;
    }
    run_removal_on_subjects(state, res, vec![RemoveSubjectRef::Object(r)], kind, count)
}

/// Resumes [`AwaitingOp::RemoveCountersChoice`] after a `Pending::ChooseCastMode` answer.
pub(super) fn resume_generic_mode(
    state: &mut GameState,
    res: &mut Resolution,
    plan: RemoveCountersPlan,
    index: usize,
) -> Flow {
    let RemoveCountersStep::PickSingleKind { subject, kinds } = plan.step else {
        unreachable!("ChooseCastMode only answers PickSingleKind")
    };
    let kind = *kinds.get(index).expect("validated counter-kind choice");
    remove_from_subject(state, res.controller, subject, kind, 1);
    res.pc += 1;
    run(state, res)
}

/// Resumes [`AwaitingOp::RemoveCountersChoice`] after a `Pending::ChooseNumber` answer.
pub(super) fn resume_generic_number(
    state: &mut GameState,
    res: &mut Resolution,
    plan: RemoveCountersPlan,
    number: u32,
) -> Flow {
    let RemoveCountersStep::ChooseCount {
        queue,
        index,
        mut remaining_budget,
        exact_required,
    } = plan.step
    else {
        unreachable!("ChooseNumber only answers ChooseCount")
    };
    let (subj, k) = queue[index];
    let avail = subject_counter_count(state, subj, k);
    let max_here = avail.min(remaining_budget.unwrap_or(u16::MAX));
    let chosen = u16::try_from(number).unwrap_or(u16::MAX).min(max_here);
    remove_from_subject(state, res.controller, subj, k, chosen);
    if let Some(rem) = &mut remaining_budget {
        *rem = rem.saturating_sub(chosen);
    }
    if let Some(pending) = advance_remove_count(
        state,
        res,
        queue,
        index + 1,
        remaining_budget,
        exact_required,
    ) {
        return Flow::Wait(pending);
    }
    res.pc += 1;
    run(state, res)
}
