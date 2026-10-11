//! Fixed-count sacrifice payments and consequences based on the departing object.

use super::{
    AwaitingOp, ChoicePrompt, Effect, Flow, Pending, PlayerRel, Resolution, YesNoPrompt, chosen,
    players_of, resolving_ability, run, run_fallback, run_nested,
};
use crate::state::GameState;
use baylee_cards_dsl::Filter;

pub(super) fn offer_payment(
    state: &mut GameState,
    res: &mut Resolution,
    who: PlayerRel,
    filter: &'static Filter,
    count: u8,
    effect: &'static Effect,
) -> Option<Pending> {
    let player = players_of(who, state, res.controller, res)
        .first()
        .copied()?;
    let options = chosen::options(state, player, filter, res.controller, res.source);
    if options.len() < usize::from(count) {
        return run_nested(state, res, std::slice::from_ref(effect));
    }
    res.awaiting = Some(AwaitingOp::SacrificePayment {
        player,
        options,
        count,
        effect,
    });
    Some(Pending::YesNo {
        player,
        prompt: YesNoPrompt::MayDo,
        source: resolving_ability(state, res),
    })
}

pub(super) fn answer_payment(state: &mut GameState, res: &mut Resolution, answer: bool) -> Flow {
    let Some(AwaitingOp::SacrificePayment {
        player,
        options,
        count,
        effect,
    }) = res.awaiting.take()
    else {
        unreachable!("fixed-count sacrifice payment")
    };
    if !answer {
        return run_fallback(state, res, std::slice::from_ref(effect));
    }
    if count == 0 {
        res.pc += 1;
        return run(state, res);
    }
    let pending = Pending::ChooseCards {
        player,
        options: options.clone(),
        min: count,
        max: count,
        prompt: ChoicePrompt::CostSacrifice,
        total: None,
    };
    res.awaiting = Some(AwaitingOp::SacrificePayment {
        player,
        options,
        count,
        effect,
    });
    Flow::Wait(pending)
}

pub(super) fn offer_conditional(
    state: &GameState,
    res: &mut Resolution,
    filter: &'static Filter,
    check: &'static Filter,
    effect: &'static Effect,
) -> Option<Pending> {
    let options = chosen::options(state, res.controller, filter, res.controller, res.source);
    if options.is_empty() {
        return None;
    }
    res.awaiting = Some(AwaitingOp::SacrificeAndIf { check, effect });
    Some(Pending::ChooseCards {
        player: res.controller,
        options,
        min: 1,
        max: 1,
        prompt: ChoicePrompt::CostSacrifice,
        total: None,
    })
}
