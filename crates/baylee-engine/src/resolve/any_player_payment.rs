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
    life: u32,
    unpaid: &'static [Effect],
    next: usize,
) -> Option<Pending> {
    let seats = state.players.len();
    for offset in next..seats {
        let seat = (usize::from(state.turn.active.get()) + offset) % seats;
        let player = state.players[seat].id;
        if state.has_left(player)
            || (life != 0 && !super::mana_life_payment::life_payable(state, player, life))
        {
            continue;
        }
        res.awaiting = Some(AwaitingOp::PlayerMayPay {
            player,
            cost,
            outcome: ManaPaymentOutcome::AnyPlayer {
                life,
                unpaid,
                next: offset + 1,
            },
        });
        let prompt = if life == 0 {
            YesNoPrompt::PayMana { cost }
        } else {
            YesNoPrompt::PayLife {
                amount: life,
                permanent: None,
            }
        };
        return Some(Pending::YesNo {
            player,
            prompt,
            source: resolving_ability(state, res),
        });
    }
    run_nested(state, res, unpaid)
}
