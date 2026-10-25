//! One optional price combines repeated mana and life atomically.
use super::{
    Amount, AwaitingOp, Effect, GameState, ManaPaymentOutcome, Pending, PlayerRel, Resolution,
    YesNoPrompt, amount2, players_of, resolving_ability,
};
use baylee_core::{ids::PlayerId, mana::ManaCost};

/// Binary repetition avoids a loop proportional to the number of counters.
/// `ManaCost`'s finite counted-symbol representation must never lower a price.
fn repeated_mana(each: ManaCost, mut times: u32) -> Option<ManaCost> {
    let expected = each.cmc().checked_mul(times)?;
    let mut power = each;
    let mut total = ManaCost::ZERO;
    while times != 0 {
        if times & 1 != 0 {
            total = total.combine(&power);
        }
        times >>= 1;
        if times != 0 {
            power = power.combine(&power);
        }
    }
    (total.cmc() == expected).then_some(total)
}

pub(crate) fn life_payable(state: &GameState, player: PlayerId, life: u32) -> bool {
    i32::try_from(life).is_ok_and(|life| state.can_pay_life(player, life))
}

#[allow(clippy::too_many_arguments)] // the printed two-part price and its two branches
pub(super) fn offer(
    state: &mut GameState,
    res: &mut Resolution,
    who: PlayerRel,
    mana: ManaCost,
    life: u32,
    times: Amount,
    paid: &'static [Effect],
    unpaid: &'static [Effect],
) -> Option<Pending> {
    let player = players_of(who, state, res.controller, res)
        .first()
        .copied()?;
    let times = amount2(&times, state, res.controller, res);
    let Some(cost) = repeated_mana(mana, times) else {
        state.numeric_failure = Some("repeated mana price exceeds the counted-symbol domain");
        return None;
    };
    let Some(life) = life.checked_mul(times) else {
        state.numeric_failure = Some("repeated life price exceeds u32");
        return None;
    };
    res.awaiting = Some(AwaitingOp::PlayerMayPay {
        player,
        cost,
        outcome: ManaPaymentOutcome::ManaAndLife { life, paid, unpaid },
    });
    Some(Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayManaAndLife { cost, life },
        source: resolving_ability(state, res),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_prices_are_exact_above_255_and_refuse_representation_overflow() {
        assert_eq!(
            repeated_mana(ManaCost::parse("{B}"), 256).unwrap().cmc(),
            256
        );
        assert_eq!(
            repeated_mana(ManaCost::parse("{B}"), 65_535).unwrap().cmc(),
            65_535
        );
        assert!(repeated_mana(ManaCost::parse("{B}{B}"), 65_535).is_none());
        assert_eq!(
            repeated_mana(ManaCost::parse("{1}{B}"), 0),
            Some(ManaCost::ZERO)
        );
    }
}
