#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn creditor_concession_prevents_future_asp_loss_but_not_existing_payment_permission() {
    let mut e = Duel::table(SEED, index::FOREST, 3)
        .battlefield(0, &[index::NAFS_ASP])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let source = asp(&e);
    attacks_hit(&mut e, vec![source]);
    priority(&mut e, P1);
    e.state.players[1].mana_pool.add(ManaColor::Colorless, 1);
    e.refresh_offer();
    let id = debts(&e)[0];
    let life = e.state.players[1].life;
    // CR 603.7e: the delayed ability retains the resolving Asp ability's
    // controller. CR 800.4d: that departed player's trigger cannot enter stack.
    e.apply(P0, PlayerAction::Concede).unwrap();
    priority(&mut e, P1);
    assert_eq!(
        debts(&e),
        vec![id],
        "the resolved payment permission persists"
    );
    pass_until(&mut e, |e| {
        e.state.turn.active == P1
            && e.state.turn.step == crate::turn::Step::Draw
            && matches!(e.pending(), Pending::Priority { .. })
    });
    assert!(stack_is_empty(&e));
    assert!(
        debts(&e).is_empty(),
        "the actual draw step closes the permission"
    );
    assert_eq!(e.state.players[1].life, life);
}
