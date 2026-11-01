//! Coin flips and replacement effects that add flips and ignore results.

use super::{
    Amount, AwaitingOp, CoinContinuation, Effect, Flow, GameEvent, GameState, Pending, PlayerId,
    PlayerRel, Resolution, YesNoPrompt, amount2, coin_stakes, life, players_of, resolving_ability,
    run, run_nested,
};

pub(super) enum Flip {
    Outcome(bool),
    Wait(Box<Pending>),
}

/// Flip once, or once more per applicable replacement ("instead flip two
/// coins and ignore one", a replacement effect, CR 614.1a), and ask which
/// results to ignore until one remains. Every result uses the seeded stream
/// and is journaled, including results later ignored.
pub(super) fn flip(
    state: &mut GameState,
    res: &mut Resolution,
    player: PlayerId,
    continuation: CoinContinuation,
) -> Flip {
    let additional = state
        .effects
        .iter()
        .filter(|fx| {
            fx.controller == player
                && matches!(
                    fx.modifier,
                    baylee_cards_dsl::Modifier::FlipTwoCoinsIgnoreOne
                )
        })
        .count();
    let mut outcomes = Vec::with_capacity(additional + 1);
    for _ in 0..=additional {
        let won = state.rng.below(2) == 0;
        state.journal.record(GameEvent::CoinFlipped { player, won });
        outcomes.push(won);
    }
    if let [won] = outcomes.as_slice() {
        return Flip::Outcome(*won);
    }
    res.awaiting = Some(AwaitingOp::CoinFlipIgnore {
        player,
        outcomes,
        continuation,
    });
    Flip::Wait(Box::new(question(state, res, player)))
}

fn question(state: &GameState, res: &Resolution, player: PlayerId) -> Pending {
    Pending::YesNo {
        player,
        prompt: YesNoPrompt::Generic,
        source: resolving_ability(state, res),
    }
}

/// Resume after "ignore this result?" answers. `true` ignores the first
/// remaining result, `false` the last; the order is deterministic and the
/// player decides each ignored result.
pub(super) fn resume(state: &mut GameState, res: &mut Resolution, ignore_first: bool) -> Flow {
    let Some(AwaitingOp::CoinFlipIgnore {
        player,
        mut outcomes,
        continuation,
    }) = res.awaiting.take()
    else {
        unreachable!("coin replacement continuation")
    };
    if ignore_first {
        outcomes.remove(0);
    } else {
        outcomes.pop();
    }
    if outcomes.len() > 1 {
        res.awaiting = Some(AwaitingOp::CoinFlipIgnore {
            player,
            outcomes,
            continuation,
        });
        return Flow::Wait(question(state, res, player));
    }
    let won = outcomes[0];
    match continuation {
        CoinContinuation::FlipBranches { won: yes, lost: no } => {
            // The replaced flip and its chosen branch are complete. Without
            // advancing, the original instruction flips again after every
            // completed branch instead of moving to the next one.
            res.pc += 1;
            match run_nested(state, res, if won { yes } else { no }) {
                Some(pending) => Flow::Wait(pending),
                None => run(state, res),
            }
        }
        CoinContinuation::UntilLossOrStop { wins } => {
            if !won {
                res.pc += 1;
                return run(state, res);
            }
            res.awaiting = Some(AwaitingOp::FlipCoinsUntilLossOrStopCumulative { wins });
            Flow::Wait(Pending::YesNo {
                player: res.controller,
                prompt: YesNoPrompt::MayDo,
                source: resolving_ability(state, res),
            })
        }
        CoinContinuation::LifeExchange => coin_stakes::resume_flip(state, res, won),
        CoinContinuation::DamageContest {
            damage,
            you_heads,
            opponent_heads,
        } => {
            let (you_heads, opponent_heads) = if you_heads.is_none() {
                (Some(won), opponent_heads)
            } else {
                (you_heads, Some(won))
            };
            if opponent_heads.is_none() {
                let continuation = CoinContinuation::DamageContest {
                    damage,
                    you_heads,
                    opponent_heads,
                };
                let opponent = *players_of(PlayerRel::Chosen, state, res.controller, res)
                    .first()
                    .expect("the contest names an opponent");
                return match flip(state, res, opponent, continuation) {
                    Flip::Outcome(won) => {
                        let finished =
                            finish_damage_contest(state, res, damage, you_heads.unwrap(), won);
                        after_contest(state, res, finished)
                    }
                    Flip::Wait(pending) => Flow::Wait(*pending),
                };
            }
            let finished = finish_damage_contest(
                state,
                res,
                damage,
                you_heads.expect("first result"),
                opponent_heads.expect("second result"),
            );
            after_contest(state, res, finished)
        }
    }
}

/// A contest finished from an answer, as `run` goes on after an instruction
/// that completed without asking: its instruction is done, and the repeat
/// [`finish_damage_contest`] may have queued right behind it, at `pc + 1`,
/// is the next one. Without the step the contest instruction ran again after
/// every contest a replacement asked about, and asked again (Krark's Thumb
/// under Mana Clash never ended). A question its damage asks keeps the
/// instruction where it is; that question's own resume moves on.
fn after_contest(state: &mut GameState, res: &mut Resolution, finished: Option<Pending>) -> Flow {
    if let Some(pending) = finished {
        return Flow::Wait(pending);
    }
    res.pc += 1;
    run(state, res)
}

pub(super) fn damage_contest(
    state: &mut GameState,
    res: &mut Resolution,
    damage: Amount,
    you_heads: Option<bool>,
    opponent_heads: Option<bool>,
) -> Option<Pending> {
    let continuation = CoinContinuation::DamageContest {
        damage,
        you_heads,
        opponent_heads,
    };
    let player = if you_heads.is_none() {
        res.controller
    } else {
        *players_of(PlayerRel::Chosen, state, res.controller, res)
            .first()
            .expect("the contest names an opponent")
    };
    match flip(state, res, player, continuation) {
        Flip::Outcome(won) => {
            let (you_heads, opponent_heads) = if you_heads.is_none() {
                (Some(won), opponent_heads)
            } else {
                (you_heads, Some(won))
            };
            if opponent_heads.is_none() {
                return damage_contest(state, res, damage, you_heads, opponent_heads);
            }
            finish_damage_contest(
                state,
                res,
                damage,
                you_heads.expect("first result"),
                opponent_heads.expect("second result"),
            )
        }
        Flip::Wait(pending) => Some(*pending),
    }
}

fn finish_damage_contest(
    state: &mut GameState,
    res: &mut Resolution,
    damage: Amount,
    you_heads: bool,
    opponent_heads: bool,
) -> Option<Pending> {
    let you = res.controller;
    let opponent = *players_of(PlayerRel::Chosen, state, you, res)
        .first()
        .expect("the contest names an opponent");
    let amount = amount2(&damage, state, you, res);
    if !(you_heads && opponent_heads) && res.effects.len() < super::immediate::REPEAT_CAP as usize {
        res.effects.insert(
            res.pc + 1,
            Effect::FlipCoinsDamageTailsRepeatUntilBothHeads { damage },
        );
    }
    let mut shares = Vec::new();
    if !you_heads {
        shares.push((you, amount));
    }
    if !opponent_heads {
        shares.push((opponent, amount));
    }
    life::deal_to_players(state, res, &shares)
}
