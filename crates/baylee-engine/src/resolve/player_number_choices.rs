//! Player-selection, secret-number, secret-guess, and cyclic-payment resolution
//! operations (`Effect::ChoosePlayerEx`, `Effect::ChooseNumberAtRandom`,
//! `Effect::SecretChooseNumbers`, `Effect::SecretNumberGuess`,
//! `Effect::RepeatEachPlayerPayUntilNone`).

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::choice::NumberPrompt;
use baylee_cards_dsl::{PlayerFilter, SecretNumberChoice, VariablePayKind};
use std::collections::VecDeque;

/// Living players in turn order starting with `start`.
pub(super) fn turn_order_starting_with(state: &GameState, start: PlayerId) -> Vec<PlayerId> {
    let n = u8::try_from(state.players.len()).unwrap_or(u8::MAX);
    if n == 0 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(usize::from(n));
    for offset in 0..n {
        let seat = PlayerId::new((start.get() + offset) % n);
        if !state.has_left(seat) {
            out.push(seat);
        }
    }
    out
}

/// Evaluates `PlayerFilter` into deduplicated, living candidate seats in APNAP order.
fn evaluate_player_filter(
    state: &GameState,
    res: &Resolution,
    filter: PlayerFilter,
) -> Vec<PlayerId> {
    let you = res.controller;
    let raw: Vec<PlayerId> = match filter {
        PlayerFilter::Among(rel) => players_of(rel, state, you, res),
        PlayerFilter::OpponentControlsMoreThanYou(obj_filter) => {
            let context = res.rule_context();
            let my_count = eval::count_controlled_by(state, you, you, obj_filter, context);
            state
                .players
                .iter()
                .filter(|p| !state.has_left(p.id) && state.is_opponent(p.id, you))
                .filter(|p| {
                    eval::count_controlled_by(state, p.id, you, obj_filter, context) > my_count
                })
                .map(|p| p.id)
                .collect()
        }
        PlayerFilter::OpponentWithGreatestLife => {
            let max_life = state
                .players
                .iter()
                .filter(|p| !state.has_left(p.id) && state.is_opponent(p.id, you))
                .map(|p| p.life)
                .max();
            match max_life {
                Some(best) => state
                    .players
                    .iter()
                    .filter(|p| {
                        !state.has_left(p.id) && state.is_opponent(p.id, you) && p.life == best
                    })
                    .map(|p| p.id)
                    .collect(),
                None => Vec::new(),
            }
        }
        PlayerFilter::NotYetRemembered(rel) => players_of(rel, state, you, res)
            .into_iter()
            .filter(|p| !res.subject.remembered_players.contains(p))
            .collect(),
        PlayerFilter::OpponentNotAttackedBySourceLastCombat => {
            let src_ref = source_version(state, res)
                .map(|version| baylee_core::ids::DamageSourceRef {
                    object: res.source,
                    version,
                })
                .or_else(|| state.source_identity(res.source));
            let last_attacked = state
                .players
                .get(usize::from(you.get()))
                .map_or(&[][..], |p| p.last_combat_attacked.as_slice());
            state
                .players
                .iter()
                .filter(|p| !state.has_left(p.id) && state.is_opponent(p.id, you))
                .filter(|p| src_ref.is_none_or(|sr| !last_attacked.contains(&(sr, p.id))))
                .map(|p| p.id)
                .collect()
        }
    };
    let mut seats = super::wave12a::apnap_seats_of(state, raw);
    seats.retain(|p| !state.has_left(*p));
    let mut deduped = Vec::with_capacity(seats.len());
    for p in seats {
        if !deduped.contains(&p) {
            deduped.push(p);
        }
    }
    deduped
}

/// Start [`Effect::ChoosePlayerEx`].
#[allow(clippy::too_many_arguments)]
pub(super) fn choose_player_ex_start(
    state: &mut GameState,
    res: &mut Resolution,
    chooser: PlayerRel,
    filter: PlayerFilter,
    random: bool,
    remember: bool,
    if_none: &'static [Effect],
    then: &'static [Effect],
) -> Option<Pending> {
    let candidates = evaluate_player_filter(state, res, filter);
    if candidates.is_empty() {
        res.chosen_player = None;
        return run_nested(state, res, if_none);
    }
    if random || candidates.len() == 1 {
        let chosen = if candidates.len() == 1 {
            candidates[0]
        } else {
            let idx = state.rng.below(candidates.len() as u64) as usize;
            candidates[idx]
        };
        res.chosen_player = Some(chosen);
        if remember && !res.subject.remembered_players.contains(&chosen) {
            res.subject.remembered_players.push(chosen);
        }
        return run_nested(state, res, then);
    }
    let chooser_seat = players_of(chooser, state, res.controller, res)
        .into_iter()
        .find(|&p| !state.has_left(p))
        .unwrap_or(res.controller);
    if state.has_left(chooser_seat) {
        res.chosen_player = None;
        return run_nested(state, res, if_none);
    }
    res.awaiting = Some(AwaitingOp::ChoosePlayerEx { remember, then });
    Some(Pending::ChoosePlayer {
        player: chooser_seat,
        options: candidates,
    })
}

/// Resume [`Effect::ChoosePlayerEx`] with `chosen`.
pub(super) fn resume_choose_player_ex(
    state: &mut GameState,
    res: &mut Resolution,
    chosen: PlayerId,
    remember: bool,
    then: &'static [Effect],
) -> Flow {
    res.chosen_player = Some(chosen);
    if remember && !res.subject.remembered_players.contains(&chosen) {
        res.subject.remembered_players.push(chosen);
    }
    if !then.is_empty() {
        let next = res.pc + 1;
        res.effects.splice(next..next, super::flatten(then));
    }
    super::advance_program(res);
    super::run(state, res)
}

/// Execute [`Effect::ChooseNumberAtRandom`].
pub(super) fn choose_number_at_random(
    state: &mut GameState,
    res: &mut Resolution,
    min: Amount,
    max: Amount,
    then: &'static [Effect],
) -> Option<Pending> {
    let you = res.controller;
    let lo = amount2(&min, state, you, res);
    let hi = amount2(&max, state, you, res);
    if lo > hi {
        return None;
    }
    let picked = if lo == hi {
        lo
    } else {
        let span = u64::from(hi - lo) + 1;
        lo.saturating_add(u32::try_from(state.rng.below(span)).unwrap_or(0))
    };
    res.subject.chosen_number = Some(picked);
    run_nested(state, res, then)
}

/// Pops the next queued `(player, number, body)` step onto `res.effects`
/// (always re-appending `op` so the final exit restores outer bindings),
/// binding `PlayerRel::EventPlayer` and `Amount::ResolutionChosenNumber`
/// without clobbering `res.chosen_player`. Returns whether any step was spliced.
fn splice_next_queued_step(res: &mut Resolution, op: Effect) -> bool {
    while let Some((seat, num, body)) = res.subject.queued_player_number_steps.pop_front() {
        if body.is_empty() {
            continue;
        }
        if res.subject.queued_player_number_saved.is_none() {
            res.subject.queued_player_number_saved =
                Some((res.subject.event_player(), res.subject.chosen_number));
        }
        res.subject.set_event_player(Some(seat));
        res.subject.chosen_number = Some(num);
        let mut program = super::flatten(body);
        program.push(op);
        let next = res.pc + 1;
        res.effects.splice(next..next, program);
        return true;
    }
    if let Some((saved_event_player, saved_chosen_number)) =
        res.subject.queued_player_number_saved.take()
    {
        res.subject.set_event_player(saved_event_player);
        res.subject.chosen_number = saved_chosen_number;
    }
    false
}

/// Execute or continue [`Effect::SecretChooseNumbers`].
pub(super) fn secret_choose_numbers_exec(
    state: &mut GameState,
    res: &mut Resolution,
    op: Effect,
    spec: SecretNumberChoice,
) -> Option<Pending> {
    if !res.subject.queued_player_number_steps.is_empty()
        || res.subject.queued_player_number_saved.is_some()
    {
        splice_next_queued_step(res, op);
        return None;
    }
    let you = res.controller;
    let seats: Vec<PlayerId> =
        super::wave12a::apnap_seats_of(state, players_of(spec.players, state, you, res))
            .into_iter()
            .filter(|p| !state.has_left(*p))
            .collect();
    let mut remaining = VecDeque::new();
    for p in seats {
        if !remaining.contains(&p) {
            remaining.push_back(p);
        }
    }
    let first = remaining.pop_front()?;
    let lo = amount2(&spec.min, state, you, res);
    let hi = amount2(&spec.max, state, you, res);
    if lo > hi {
        return None;
    }
    res.awaiting = Some(AwaitingOp::SecretNumbers {
        current: first,
        remaining,
        chosen: Vec::new(),
        min: lo,
        max: hi,
        spec,
        op,
    });
    Some(Pending::ChooseNumber {
        player: first,
        min: lo,
        max: hi,
        reason: NumberPrompt::Choice,
    })
}

/// Resume [`Effect::SecretChooseNumbers`] with `number` chosen by `current`.
#[allow(clippy::too_many_arguments)]
pub(super) fn resume_secret_numbers(
    state: &mut GameState,
    res: &mut Resolution,
    number: u32,
    current: PlayerId,
    mut remaining: VecDeque<PlayerId>,
    mut chosen: Vec<(PlayerId, u32)>,
    min: u32,
    max: u32,
    spec: SecretNumberChoice,
    op: Effect,
) -> Flow {
    if !state.has_left(current) {
        chosen.push((current, number));
    }
    while let Some(next_seat) = remaining.pop_front() {
        if state.has_left(next_seat) {
            continue;
        }
        res.awaiting = Some(AwaitingOp::SecretNumbers {
            current: next_seat,
            remaining,
            chosen,
            min,
            max,
            spec,
            op,
        });
        return Flow::Wait(Pending::ChooseNumber {
            player: next_seat,
            min,
            max,
            reason: NumberPrompt::Choice,
        });
    }
    if !chosen.is_empty() {
        let max_val = chosen.iter().map(|&(_, v)| v).max().unwrap_or(0);
        let min_val = chosen.iter().map(|&(_, v)| v).min().unwrap_or(0);
        if spec.remember_highest {
            for &(p, v) in &chosen {
                if v == max_val && !res.subject.remembered_players.contains(&p) {
                    res.subject.remembered_players.push(p);
                }
            }
        }
        if !spec.on_each.is_empty() {
            for &(p, v) in &chosen {
                res.subject
                    .queued_player_number_steps
                    .push_back((p, v, spec.on_each));
            }
        }
        if !spec.on_highest.is_empty() {
            for &(p, v) in &chosen {
                if v == max_val {
                    res.subject
                        .queued_player_number_steps
                        .push_back((p, v, spec.on_highest));
                }
            }
        }
        if !spec.on_lowest.is_empty() {
            for &(p, v) in &chosen {
                if v == min_val {
                    res.subject
                        .queued_player_number_steps
                        .push_back((p, v, spec.on_lowest));
                }
            }
        }
        if !spec.on_not_lowest.is_empty() {
            for &(p, v) in &chosen {
                if v != min_val {
                    res.subject
                        .queued_player_number_steps
                        .push_back((p, v, spec.on_not_lowest));
                }
            }
        }
        if chosen.len() >= 2 {
            let matched = chosen.windows(2).all(|w| w[0].1 == w[1].1);
            if matched && !spec.on_matched.is_empty() {
                res.subject.queued_player_number_steps.push_back((
                    res.controller,
                    chosen[0].1,
                    spec.on_matched,
                ));
            } else if !matched && !spec.on_unmatched.is_empty() {
                res.subject.queued_player_number_steps.push_back((
                    res.controller,
                    chosen[0].1,
                    spec.on_unmatched,
                ));
            }
        }
        splice_next_queued_step(res, op);
    }
    super::advance_program(res);
    super::run(state, res)
}

fn available_secret_guess_numbers(
    state: &GameState,
    res: &Resolution,
    min: u8,
    max: u8,
    track_used_on_source: bool,
) -> Vec<u8> {
    if min > max {
        return Vec::new();
    }
    // CR 400.7: only read tracked used numbers from the exact source incarnation.
    let used = if track_used_on_source {
        super::subjects::source(state, res)
            .filter(|src| super::subjects::is_current(state, *src))
            .and_then(|src| state.object(src.object))
            .map_or(0, crate::object::GameObject::used_chosen_numbers)
    } else {
        0
    };
    (min..=max)
        .filter(|&n| n >= 32 || (used & (1_u32 << n)) == 0)
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn ask_secret_guess_choose(
    state: &GameState,
    res: &mut Resolution,
    guesser: PlayerId,
    min: u8,
    max: u8,
    track_used_on_source: bool,
    on_correct: &'static [Effect],
    on_wrong: &'static [Effect],
) -> Option<Pending> {
    let allowed = available_secret_guess_numbers(state, res, min, max, track_used_on_source);
    if allowed.is_empty() {
        return None;
    }
    let options: Vec<crate::choice::CastModeDesc> = allowed
        .iter()
        .enumerate()
        .map(|(i, &num)| crate::choice::CastModeDesc {
            index: u32::try_from(i).unwrap_or(u32::MAX),
            kind: crate::choice::CastModeKind::Mode(usize::from(num)),
            cost: baylee_core::mana::ManaCost::ZERO,
        })
        .collect();
    res.awaiting = Some(AwaitingOp::SecretGuessChoose {
        guesser,
        min,
        max,
        allowed,
        track_used_on_source,
        on_correct,
        on_wrong,
    });
    Some(Pending::ChooseCastMode {
        player: res.controller,
        object: res.source,
        options,
    })
}

/// Start [`Effect::SecretNumberGuess`] (*The Toymaker's Trap*).
#[allow(clippy::too_many_arguments)]
pub(super) fn secret_number_guess_start(
    state: &mut GameState,
    res: &mut Resolution,
    min: u8,
    max: u8,
    track_used_on_source: bool,
    guesser: PlayerRel,
    on_correct: &'static [Effect],
    on_wrong: &'static [Effect],
) -> Option<Pending> {
    let you = res.controller;
    if state.has_left(you) {
        return None;
    }
    let allowed = available_secret_guess_numbers(state, res, min, max, track_used_on_source);
    if allowed.is_empty() {
        return None;
    }
    let guessers: Vec<PlayerId> =
        super::wave12a::apnap_seats_of(state, players_of(guesser, state, you, res))
            .into_iter()
            .filter(|p| !state.has_left(*p))
            .collect();
    match guessers.as_slice() {
        [] => None,
        [only] => ask_secret_guess_choose(
            state,
            res,
            *only,
            min,
            max,
            track_used_on_source,
            on_correct,
            on_wrong,
        ),
        _ => {
            res.awaiting = Some(AwaitingOp::SecretGuessPickGuesser {
                min,
                max,
                track_used_on_source,
                on_correct,
                on_wrong,
            });
            Some(Pending::ChoosePlayer {
                player: you,
                options: guessers,
            })
        }
    }
}

/// Resume choosing which opponent guesses in [`Effect::SecretNumberGuess`].
#[allow(clippy::too_many_arguments)]
pub(super) fn resume_secret_guess_guesser(
    state: &mut GameState,
    res: &mut Resolution,
    guesser: PlayerId,
    min: u8,
    max: u8,
    track_used_on_source: bool,
    on_correct: &'static [Effect],
    on_wrong: &'static [Effect],
) -> Flow {
    if let Some(pending) = ask_secret_guess_choose(
        state,
        res,
        guesser,
        min,
        max,
        track_used_on_source,
        on_correct,
        on_wrong,
    ) {
        return Flow::Wait(pending);
    }
    super::advance_program(res);
    super::run(state, res)
}

/// Resume controller's secret number selection in [`Effect::SecretNumberGuess`].
#[allow(clippy::too_many_arguments)]
pub(super) fn resume_secret_guess_choose(
    state: &mut GameState,
    res: &mut Resolution,
    option_index: usize,
    guesser: PlayerId,
    min: u8,
    max: u8,
    allowed: &[u8],
    track_used_on_source: bool,
    on_correct: &'static [Effect],
    on_wrong: &'static [Effect],
) -> Flow {
    let Some(&secret) = allowed.get(option_index) else {
        super::advance_program(res);
        return super::run(state, res);
    };
    // CR 400.7: only record used numbers on the exact source incarnation.
    if track_used_on_source
        && let Some(src) = super::subjects::source(state, res)
            .filter(|src| super::subjects::is_current(state, *src))
        && let Some(obj) = state.object_mut(src.object)
    {
        obj.mark_used_chosen_number(secret);
    }
    if state.has_left(guesser) {
        super::advance_program(res);
        return super::run(state, res);
    }
    res.awaiting = Some(AwaitingOp::SecretGuessAnswer {
        guesser,
        secret,
        on_correct,
        on_wrong,
    });
    Flow::Wait(Pending::ChooseNumber {
        player: guesser,
        min: u32::from(min),
        max: u32::from(max),
        reason: NumberPrompt::Choice,
    })
}

/// Resume guesser's guess in [`Effect::SecretNumberGuess`].
pub(super) fn resume_secret_guess_answer(
    state: &mut GameState,
    res: &mut Resolution,
    guess: u32,
    guesser: PlayerId,
    secret: u8,
    on_correct: &'static [Effect],
    on_wrong: &'static [Effect],
) -> Flow {
    res.chosen_player = Some(guesser);
    res.subject.chosen_number = Some(guess);
    let branch = if guess == u32::from(secret) {
        on_correct
    } else {
        on_wrong
    };
    if !branch.is_empty() {
        let next = res.pc + 1;
        res.effects.splice(next..next, super::flatten(branch));
    }
    super::advance_program(res);
    super::run(state, res)
}

fn variable_pay_max(
    state: &GameState,
    res: &Resolution,
    player: PlayerId,
    kind: VariablePayKind,
    max_per_player: Option<Amount>,
) -> u32 {
    let cap = max_per_player.map_or(u32::MAX, |amt| {
        super::amount_for_player(&amt, state, res.controller, res, player)
    });
    match kind {
        VariablePayKind::Life => {
            let avail = u32::try_from(state.life_payable(player).max(0)).unwrap_or(0);
            cap.min(avail)
        }
        VariablePayKind::Energy => {
            let avail = state
                .players
                .get(usize::from(player.get()))
                .map_or(0, |p| u32::from(p.energy));
            cap.min(avail)
        }
        VariablePayKind::GenericMana => {
            let pool_max = state.players.get(usize::from(player.get())).map_or(0, |p| {
                u32::try_from(p.mana_pool.total()).unwrap_or(u32::MAX)
            });
            cap.min(pool_max)
        }
    }
}

fn variable_pay_reason(kind: VariablePayKind) -> NumberPrompt {
    match kind {
        VariablePayKind::Energy => NumberPrompt::VariableEnergyPayment,
        VariablePayKind::GenericMana => NumberPrompt::VariableManaPayment,
        VariablePayKind::Life => NumberPrompt::VariableLifePayment,
    }
}

#[allow(clippy::too_many_arguments)]
fn ask_next_repeat_payer(
    state: &mut GameState,
    res: &mut Resolution,
    mut paid_any_in_cycle: bool,
    mut remaining_in_cycle: VecDeque<PlayerId>,
    totals: Vec<(PlayerId, u32)>,
    kind: VariablePayKind,
    max_per_player: Option<Amount>,
    then_per_player: &'static [Effect],
    op: Effect,
) -> Option<Pending> {
    loop {
        while let Some(seat) = remaining_in_cycle.pop_front() {
            if state.has_left(seat) {
                continue;
            }
            let max = variable_pay_max(state, res, seat, kind, max_per_player);
            if max == 0 {
                continue;
            }
            res.awaiting = Some(AwaitingOp::RepeatPayUntilNone {
                current: seat,
                paid_any_in_cycle,
                remaining_in_cycle,
                totals,
                kind,
                max_per_player,
                then_per_player,
                op,
            });
            return Some(Pending::ChooseNumber {
                player: seat,
                min: 0,
                max,
                reason: variable_pay_reason(kind),
            });
        }
        if !paid_any_in_cycle {
            // Nobody paid in a full cycle around the table: finalize.
            if !then_per_player.is_empty() {
                for seat in turn_order_starting_with(state, res.controller) {
                    let total = totals
                        .iter()
                        .find_map(|&(p, amt)| (p == seat).then_some(amt))
                        .unwrap_or(0);
                    if total > 0 {
                        res.subject.queued_player_number_steps.push_back((
                            seat,
                            total,
                            then_per_player,
                        ));
                    }
                }
            }
            return None;
        }
        paid_any_in_cycle = false;
        remaining_in_cycle = turn_order_starting_with(state, res.controller).into();
    }
}

/// Execute or continue [`Effect::RepeatEachPlayerPayUntilNone`] (*Plague of Vermin*).
pub(super) fn repeat_pay_until_none_exec(
    state: &mut GameState,
    res: &mut Resolution,
    op: Effect,
    kind: VariablePayKind,
    max_per_player: Option<Amount>,
    then_per_player: &'static [Effect],
) -> Option<Pending> {
    if !res.subject.queued_player_number_steps.is_empty()
        || res.subject.queued_player_number_saved.is_some()
    {
        splice_next_queued_step(res, op);
        return None;
    }
    let cycle: VecDeque<PlayerId> = turn_order_starting_with(state, res.controller).into();
    let pending = ask_next_repeat_payer(
        state,
        res,
        false,
        cycle,
        Vec::new(),
        kind,
        max_per_player,
        then_per_player,
        op,
    );
    if pending.is_none() {
        splice_next_queued_step(res, op);
    }
    pending
}

/// Resume [`Effect::RepeatEachPlayerPayUntilNone`] with `amount` paid by `current`.
#[allow(clippy::too_many_arguments)]
pub(super) fn resume_repeat_pay_until_none(
    state: &mut GameState,
    res: &mut Resolution,
    amount: u32,
    current: PlayerId,
    mut paid_any_in_cycle: bool,
    remaining_in_cycle: VecDeque<PlayerId>,
    mut totals: Vec<(PlayerId, u32)>,
    kind: VariablePayKind,
    max_per_player: Option<Amount>,
    then_per_player: &'static [Effect],
    op: Effect,
) -> Flow {
    if amount > 0 && !state.has_left(current) {
        let paid = match kind {
            VariablePayKind::Life => {
                if let Ok(life) = i32::try_from(amount)
                    && state.can_pay_life(current, life)
                {
                    state.change_life(current, -life, Cause::Cost);
                    true
                } else {
                    false
                }
            }
            VariablePayKind::Energy => state.pay_energy(current, amount),
            VariablePayKind::GenericMana => {
                let cost = baylee_core::mana::ManaCost::from_symbol_generic(amount);
                let what = super::payment_for(state, res);
                crate::casting::pay_mana_for(state, current, what, &cost).is_some()
            }
        };
        if paid {
            paid_any_in_cycle = true;
            if let Some((_, sum)) = totals.iter_mut().find(|(p, _)| *p == current) {
                *sum = sum.saturating_add(amount);
            } else {
                totals.push((current, amount));
            }
        }
    }
    if let Some(pending) = ask_next_repeat_payer(
        state,
        res,
        paid_any_in_cycle,
        remaining_in_cycle,
        totals,
        kind,
        max_per_player,
        then_per_player,
        op,
    ) {
        return Flow::Wait(pending);
    }
    splice_next_queued_step(res, op);
    super::advance_program(res);
    super::run(state, res)
}
