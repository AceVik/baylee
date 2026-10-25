//! APNAP payment offers stop after the first complete payment.
use super::{
    AwaitingOp, Effect, GameState, ManaPaymentOutcome, Pending, Resolution, YesNoPrompt,
    resolving_ability, run_nested,
};
use baylee_core::mana::ManaCost;

pub(super) fn offer(
    state: &mut GameState,
    res: &mut Resolution,
    cost: ManaCost,
    unpaid: &'static [Effect],
    next: usize,
) -> Option<Pending> {
    let seats = state.players.len();
    for offset in next..seats {
        let seat = (usize::from(state.turn.active.get()) + offset) % seats;
        let player = state.players[seat].id;
        if state.has_left(player) {
            continue;
        }
        res.awaiting = Some(AwaitingOp::PlayerMayPay {
            player,
            cost,
            outcome: ManaPaymentOutcome::AnyPlayer {
                unpaid,
                next: offset + 1,
            },
        });
        return Some(Pending::YesNo {
            player,
            prompt: YesNoPrompt::PayMana { cost },
            source: resolving_ability(state, res),
        });
    }
    run_nested(state, res, unpaid)
}
