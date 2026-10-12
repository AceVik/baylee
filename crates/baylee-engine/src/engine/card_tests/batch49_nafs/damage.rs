#[allow(clippy::wildcard_imports)]
use super::*;
#[test]
fn prevented_asp_combat_damage_creates_no_debt() {
    let mut e = prepared(&[index::FOG], &[index::NAFS_ASP], &[]);
    let source = asp(&e);
    let life = e.state.players[1].life;
    cast(&mut e, P0, index::FOG);
    pass_until(&mut e, stack_is_empty);
    attacks_hit(&mut e, vec![source]);
    assert_eq!(e.state.players[1].life, life);
    priority(&mut e, P1);
    e.state.players[1].mana_pool.add(ManaColor::Colorless, 2);
    e.refresh_offer();
    assert!(debts(&e).is_empty());
    p1_upkeep(&mut e);
    let turn = e.state.turn.number;
    pass_until(&mut e, |e| {
        e.state.turn.number == turn
            && e.state.turn.step == crate::turn::Step::Draw
            && matches!(e.pending(), Pending::Priority { .. })
    });
    assert!(stack_is_empty(&e));
    assert_eq!(e.state.players[1].life, life);
}
#[test]
fn actual_agatha_granted_ping_is_asp_noncombat_damage_and_two_hits_create_two_debts() {
    let mut e = prepared(
        &[index::PRODIGAL_SORCERER, index::TWIDDLE],
        &[index::NAFS_ASP, index::AGATHA_S_SOUL_CAULDRON],
        &[],
    );
    let source = asp(&e);
    let donor = in_hand(&e, P0, index::PRODIGAL_SORCERER).unwrap();
    // Transparent graveyard fixture; all acquisitions, counters, damage and untap are actual card actions.
    e.state
        .move_object(
            donor,
            ZoneLocation::Graveyard(P0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.sync_static_effects();
    e.refresh_offer();
    activate(&mut e, P0, index::AGATHA_S_SOUL_CAULDRON, 2);
    targets(&mut e, vec![donor], vec![]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    targets(&mut e, vec![source], vec![]);
    pass_until(&mut e, stack_is_empty);
    let abilities: Vec<_> = crate::effects::granted_activated(e.state(), source)
        .map(|a| a.id)
        .collect();
    assert_eq!(abilities.len(), 1);
    let life = e.state.players[1].life;
    for hit in 0..2 {
        priority(&mut e, P0);
        e.refresh_offer();
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source,
                ability_index: abilities[0],
            },
        )
        .unwrap();
        targets(&mut e, vec![], vec![P1]);
        pass_until(&mut e, stack_is_empty);
        if hit == 0 {
            cast(&mut e, P0, index::TWIDDLE);
            targets(&mut e, vec![source], vec![]);
            pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
            e.apply(P0, PlayerAction::YesNo(true)).unwrap();
            pass_until(&mut e, stack_is_empty);
        }
    }
    assert_eq!(e.state.players[1].life, life - 2);
    priority(&mut e, P1);
    e.state.players[1].mana_pool.add(ManaColor::Colorless, 2);
    e.refresh_offer();
    let ids = debts(&e);
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1]);
    draw_trigger(&mut e);
    assert_eq!(e.state.zones.list(ZoneLocation::Stack).len(), 2);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[1].life, life - 4);
}
