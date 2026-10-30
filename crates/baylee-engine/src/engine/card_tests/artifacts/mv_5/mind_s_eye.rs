//! Mind's Eye lets its controller pay for a card on an opponent's draw.

use super::*;

fn mind_s_eye() -> CardIndex {
    card_index("63fd2a57-7a47-4e07-947c-f4e9da7ee538")
}

#[test]
fn mind_s_eye_draws_when_the_triggered_payment_is_made() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[mind_s_eye(), island()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let before = e.state().zones.list(ZoneLocation::Hand(p0)).len();
    pass_until(&mut e, |e| {
        e.state().turn.active == p1
            && matches!(
                e.pending(),
                Pending::YesNo {
                    prompt: YesNoPrompt::PayTax { .. },
                    ..
                }
            )
    });
    let player = e.pending().asked().unwrap();
    assert_eq!(player, p0);
    e.apply(p0, PlayerAction::YesNo(true)).unwrap();
    if matches!(e.pending(), Pending::Priority { .. })
        && e.state().players[0].mana_pool.total() == 0
    {
        tap_all_mana(&mut e, p0);
        e.apply(p0, PlayerAction::PassPriority).unwrap();
    }
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(p0)).len(),
        before + 1
    );
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
}
