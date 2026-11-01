//! Winner-controlled coin contests, with replacements settled between life instructions.
use super::{
    Amount, AwaitingOp, Cause, Flow, GameState, Pending, PlayerId, PlayerRel, Resolution,
    YesNoPrompt, amount2, life, players_of, resolving_ability, run,
};

#[derive(Clone, Copy, Debug, Hash)]
enum Stage {
    Flip,
    FirstLifeChange,
    SecondLifeChange,
    Ask,
}

/// Frozen operands while a life-gain replacement asks a question.
#[derive(Clone, Debug, Hash)]
pub struct CoinStake {
    opponent: PlayerId,
    stake: u32,
    multiplier: u32,
    won: bool,
    stage: Stage,
}

/// A coin winner's optional decision, retaining its original opponent requirement.
#[derive(Clone, Debug, Hash)]
pub struct CoinDecision {
    opponent: PlayerId,
    stake: u32,
    multiplier: u32,
    chooser: PlayerId,
    opponents_only: bool,
}

pub(super) fn start(
    state: &mut GameState,
    res: &mut Resolution,
    opponent: PlayerRel,
    initial: Amount,
    multiplier: u32,
) -> Option<Pending> {
    let mut contest = if let Some(contest) = res.coin_stake.take() {
        contest
    } else {
        let opponent = players_of(opponent, state, res.controller, res)
            .first()
            .copied()?;
        if opponent == res.controller || state.has_left(opponent) {
            return None;
        }
        Box::new(CoinStake {
            opponent,
            stake: amount2(&initial, state, res.controller, res),
            multiplier,
            won: false,
            stage: Stage::Flip,
        })
    };
    let Ok(life) = i32::try_from(contest.stake) else {
        state.numeric_failure = Some("coin-contest life stake exceeds i32");
        return None;
    };
    match contest.stage {
        Stage::Flip => {
            match super::coin::flip(
                state,
                res,
                res.controller,
                super::CoinContinuation::LifeExchange,
            ) {
                super::coin::Flip::Outcome(won) => contest.won = won,
                super::coin::Flip::Wait(pending) => {
                    // A replacement asks which result to keep (CR 614.1a):
                    // the contest waits at its flip, with its opponent and
                    // stake, for `resume_flip` to hand it the kept one.
                    res.coin_stake = Some(contest);
                    return Some(*pending);
                }
            }
            contest.stage = Stage::FirstLifeChange;
        }
        Stage::FirstLifeChange => {
            if contest.won {
                life::gain_life(state, res.controller, life);
            } else {
                state.change_life(res.controller, -life, Cause::Effect);
            }
            contest.stage = Stage::SecondLifeChange;
        }
        Stage::SecondLifeChange => {
            if contest.won {
                state.change_life(contest.opponent, -life, Cause::Effect);
            } else {
                life::gain_life(state, contest.opponent, life);
            }
            contest.stage = Stage::Ask;
        }
        Stage::Ask => {
            let winner = if contest.won {
                res.controller
            } else {
                contest.opponent
            };
            res.awaiting = Some(AwaitingOp::RepeatCoinLifeStake(CoinDecision {
                opponent: contest.opponent,
                stake: contest.stake,
                multiplier: contest.multiplier,
                chooser: winner,
                opponents_only: state.is_opponent(winner, res.controller),
            }));
            return question(state, res);
        }
    }
    // The interpreter handles queued draw replacements before re-entering this instruction.
    res.coin_stake = Some(contest);
    None
}

/// The result a replacement kept for the contest's flip (CR 614.1a): the
/// contest waiting at its flip goes on from it, to its life changes. It is
/// not flipped again, and its stake and opponent are the ones it had.
pub(super) fn resume_flip(state: &mut GameState, res: &mut Resolution, won: bool) -> Flow {
    if let Some(contest) = res.coin_stake.as_deref_mut() {
        contest.won = won;
        contest.stage = Stage::FirstLifeChange;
    }
    // The instruction is still the one at `pc`, and `start` takes the
    // contest back up from `res.coin_stake`.
    run(state, res)
}

pub(super) fn answer(state: &mut GameState, res: &mut Resolution, again: bool) -> Option<Flow> {
    if !matches!(res.awaiting, Some(AwaitingOp::RepeatCoinLifeStake(_))) {
        return None;
    }
    let Some(AwaitingOp::RepeatCoinLifeStake(decision)) = res.awaiting.take() else {
        unreachable!("matched above")
    };
    let CoinDecision {
        opponent,
        stake,
        multiplier,
        ..
    } = decision;
    if again {
        let Some(stake) = stake.checked_mul(multiplier) else {
            state.numeric_failure = Some("coin-contest life stake exceeds u32");
            return Some(Flow::Complete);
        };
        res.coin_stake = Some(Box::new(CoinStake {
            opponent,
            stake,
            multiplier,
            won: false,
            stage: Stage::Flip,
        }));
    } else {
        res.pc += 1;
    }
    Some(run(state, res))
}

fn replacement_options(
    state: &GameState,
    controller: PlayerId,
    opponents_only: bool,
) -> Vec<PlayerId> {
    let mut options: Vec<_> = state
        .players
        .iter()
        .filter(|player| !player.has_lost())
        .map(|player| player.id)
        .collect();
    if opponents_only
        && options
            .iter()
            .any(|player| state.is_opponent(*player, controller))
    {
        options.retain(|player| state.is_opponent(*player, controller));
    }
    options
}

fn question(state: &GameState, res: &mut Resolution) -> Option<Pending> {
    let (decision, replacing) = match res.awaiting.take()? {
        AwaitingOp::RepeatCoinLifeStake(decision) => (decision, false),
        AwaitingOp::ChooseCoinRepeatPlayer(decision) => (decision, true),
        other => {
            res.awaiting = Some(other);
            return None;
        }
    };
    if !replacing && !state.has_left(decision.chooser) {
        let player = decision.chooser;
        res.awaiting = Some(AwaitingOp::RepeatCoinLifeStake(decision));
        return Some(Pending::YesNo {
            player,
            prompt: YesNoPrompt::MayDo,
            source: resolving_ability(state, res),
        });
    }
    let options = replacement_options(state, res.controller, decision.opponents_only);
    if options.is_empty() {
        return None;
    }
    // CR 800.4g: the controller chooses a substitute. If that controller has left,
    // CR 800.4h assigns this rule-required choice to the next player in turn order.
    let player = if state.has_left(res.controller) {
        let n = state.players.len();
        (1..=n)
            .map(|offset| state.players[(usize::from(res.controller.get()) + offset) % n].id)
            .find(|player| !state.has_left(*player))?
    } else {
        res.controller
    };
    res.awaiting = Some(AwaitingOp::ChooseCoinRepeatPlayer(decision));
    Some(Pending::ChoosePlayer { player, options })
}

/// Refresh only this contest's choice when any player concedes.
pub(crate) fn refresh_choice(state: &GameState, res: &mut Resolution) -> Option<Pending> {
    let needs_refresh = match &res.awaiting {
        Some(AwaitingOp::RepeatCoinLifeStake(decision)) => state.has_left(decision.chooser),
        Some(AwaitingOp::ChooseCoinRepeatPlayer(_)) => true,
        _ => false,
    };
    needs_refresh.then(|| question(state, res)).flatten()
}

/// Install the substitute selected through the engine's legal `ChoosePlayer` menu.
pub(crate) fn choose_player(state: &GameState, res: &mut Resolution, chosen: PlayerId) -> Pending {
    let Some(AwaitingOp::ChooseCoinRepeatPlayer(mut decision)) = res.awaiting.take() else {
        unreachable!("coin replacement chooser")
    };
    debug_assert!(
        replacement_options(state, res.controller, decision.opponents_only).contains(&chosen)
    );
    decision.chooser = chosen;
    res.awaiting = Some(AwaitingOp::RepeatCoinLifeStake(decision));
    question(state, res).expect("the selected substitute is still playing")
}
