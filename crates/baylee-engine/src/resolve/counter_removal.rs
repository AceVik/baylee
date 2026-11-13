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
