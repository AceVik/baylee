#[allow(clippy::wildcard_imports)]
use super::*;
#[test]
fn asp_payment_before_draw_prevents_loss_but_deadline_rejects_even_with_mana() {
    for pay in [false, true] {
        let mut e = prepared(&[], &[index::NAFS_ASP], &[]);
        let source = asp(&e);
        attacks_hit(&mut e, vec![source]);
        p1_upkeep(&mut e);
        e.state.players[1].mana_pool.add(ManaColor::Colorless, 1);
        e.refresh_offer();
        let id = debts(&e)[0];
        let life = e.state.players[1].life;
        if pay {
            e.apply(P1, PlayerAction::TakeGrantedAction { id }).unwrap();
            pass_until(&mut e, |e| {
                e.state.turn.step == crate::turn::Step::Draw
                    && matches!(e.pending(), Pending::Priority { .. })
            });
            assert!(stack_is_empty(&e));
            assert_eq!(e.state.players[1].life, life);
        } else {
            draw_trigger(&mut e);
            priority(&mut e, P1);
            e.state.players[1].mana_pool.add(ManaColor::Colorless, 1);
            e.refresh_offer();
            assert!(debts(&e).is_empty());
            let before = e.snapshot_hash();
            assert!(e.apply(P1, PlayerAction::TakeGrantedAction { id }).is_err());
            assert_eq!(e.snapshot_hash(), before);
            pass_until(&mut e, stack_is_empty);
            assert_eq!(e.state.players[1].life, life - 1);
        }
    }
}
#[test]
fn actual_tidebinder_counters_unpaid_asp_draw_trigger_without_retry() {
    let mut e = prepared(&[index::TISHANA_S_TIDEBINDER], &[index::NAFS_ASP], &[]);
    let source = asp(&e);
    attacks_hit(&mut e, vec![source]);
    draw_trigger(&mut e);
    let ability = *e.state.zones.list(ZoneLocation::Stack).last().unwrap();
    let life = e.state.players[1].life;
    cast(&mut e, P0, index::TISHANA_S_TIDEBINDER);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    targets(&mut e, vec![ability], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[1].life, life);
    let turn = e.state.turn.number;
    pass_until(&mut e, |e| {
        e.state.turn.number > turn
            && e.state.turn.active == P1
            && e.state.turn.step == crate::turn::Step::Draw
            && matches!(e.pending(), Pending::Priority { .. })
    });
    assert!(stack_is_empty(&e));
    assert_eq!(e.state.players[1].life, life);
}
#[test]
fn replacing_draw_with_island_sanctuary_does_not_skip_asp_deadline() {
    let mut e = prepared(&[], &[index::NAFS_ASP], &[index::ISLAND_SANCTUARY]);
    let source = asp(&e);
    attacks_hit(&mut e, vec![source]);
    let hand = e.state.zones.list(ZoneLocation::Hand(P1)).len();
    let life = e.state.players[1].life;
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                player: P1,
                prompt: crate::choice::YesNoPrompt::MayDo,
                ..
            }
        )
    });
    e.apply(P1, PlayerAction::YesNo(true)).unwrap();
    draw_trigger(&mut e);
    assert_eq!(e.state.zones.list(ZoneLocation::Hand(P1)).len(), hand);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[1].life, life - 1);
}
#[test]
fn skipped_draw_step_fixture_retains_payment_until_next_actual_draw_step() {
    let mut e = prepared(&[], &[index::NAFS_ASP], &[]);
    let source = asp(&e);
    attacks_hit(&mut e, vec![source]);
    p1_upkeep(&mut e);
    e.state.players[1].mana_pool.add(ManaColor::Colorless, 1);
    e.refresh_offer();
    let id = debts(&e)[0];
    // Explicit scheduling fixture: no implemented card skips an entire draw step.
    // Omit this step, preserving actual turn start, cleanup, debt and next-turn progression.
    e.state.turn.phase = crate::turn::Phase::FirstMain;
    e.state.turn.step = crate::turn::Step::Main;
    e.refresh_offer();
    assert_eq!(debts(&e), vec![id]);
    let life = e.state.players[1].life;
    draw_trigger(&mut e);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[1].life, life - 1);
}
