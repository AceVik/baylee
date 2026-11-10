#[allow(clippy::wildcard_imports)]
use super::*;
#[test]
fn two_asps_create_independent_debts_pay_one_and_reject_stale_id() {
    let mut e = prepared(&[], &[index::NAFS_ASP, index::NAFS_ASP], &[]);
    let asps: Vec<_> = e
        .state
        .battlefield_seen()
        .filter(|id| {
            e.state
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == index::NAFS_ASP))
        })
        .collect();
    let life = e.state.players[1].life;
    attacks_hit(&mut e, asps);
    priority(&mut e, P0);
    assert!(
        debts(&e).is_empty(),
        "creditor cannot take damaged player's payment action"
    );
    p1_upkeep(&mut e);
    e.state.players[1].mana_pool.add(ManaColor::Colorless, 1);
    e.refresh_offer();
    let ids = debts(&e);
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1]);
    e.apply(P1, PlayerAction::TakeGrantedAction { id: ids[0] })
        .unwrap();
    assert!(stack_is_empty(&e));
    assert_eq!(e.state.players[1].mana_pool.total(), 0);
    let before = e.snapshot_hash();
    assert!(
        e.apply(P1, PlayerAction::TakeGrantedAction { id: ids[0] })
            .is_err()
    );
    assert_eq!(e.snapshot_hash(), before);
    // The other debt retains identity; insufficient mana must not forgive it.
    let before = e.snapshot_hash();
    assert!(
        e.apply(P1, PlayerAction::TakeGrantedAction { id: ids[1] })
            .is_err()
    );
    assert_eq!(e.snapshot_hash(), before);
    draw_trigger(&mut e);
    assert_eq!(e.state.zones.list(ZoneLocation::Stack).len(), 1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[1].life, life - 3);
}
#[test]
fn asp_debt_survives_cleanup_and_actual_source_destruction() {
    let mut e = prepared(&[index::TERROR], &[index::NAFS_ASP], &[]);
    let source = asp(&e);
    attacks_hit(&mut e, vec![source]);
    priority(&mut e, P1);
    e.state.players[1].mana_pool.add(ManaColor::Colorless, 1);
    e.refresh_offer();
    let id = debts(&e)[0];
    cast(&mut e, P0, index::TERROR);
    targets(&mut e, vec![source], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(source).unwrap().zone, Zone::Graveyard);
    p1_upkeep(&mut e);
    e.state.players[1].mana_pool.add(ManaColor::Colorless, 1);
    e.refresh_offer();
    assert_eq!(
        debts(&e),
        vec![id],
        "cleanup/source death cannot expire debt or regenerate its ID"
    );
    let life = e.state.players[1].life;
    draw_trigger(&mut e);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[1].life, life - 1);
}
