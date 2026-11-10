#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch47_rhystic_chooses_color_before_any_player_pays_and_never_uses_stack() {
    for payer in [None, Some(P0), Some(P1)] {
        let mut e = prepared(&[], &[index::RHYSTIC_CAVE], &[]);
        let cave = on_battlefield(&e, P0, index::RHYSTIC_CAVE).unwrap();
        set_pool(&mut e, P0, &[(ManaColor::Colorless, 1)]);
        set_pool(&mut e, P1, &[(ManaColor::Colorless, 1)]);
        activate_as(&mut e, P0, cave, 0);
        assert!(matches!(e.pending(), Pending::ChooseColor { .. }));
        assert!(stack_is_empty(&e));
        e.apply(P0, PlayerAction::ChooseColor(ManaColor::Green))
            .unwrap();
        let mut asked = Vec::new();
        while let Pending::YesNo { player, .. } = e.pending().clone() {
            assert!(!asked.contains(&player));
            asked.push(player);
            e.apply(player, PlayerAction::YesNo(payer == Some(player)))
                .unwrap();
        }
        assert!(stack_is_empty(&e));
        assert!(is_tapped(&e, cave));
        assert_eq!(
            e.state.players[0].mana_pool.available(ManaColor::Green),
            u32::from(payer.is_none())
        );
        for player in [P0, P1] {
            assert_eq!(
                e.state.players[usize::from(player.get())]
                    .mana_pool
                    .available(ManaColor::Colorless),
                u32::from(payer != Some(player))
            );
        }
        assert_eq!(
            asked.first(),
            Some(&P0),
            "active player receives the first payment option"
        );
        if payer == Some(P0) {
            assert_eq!(asked, vec![P0]);
        } else {
            assert_eq!(asked, vec![P0, P1]);
        }
    }
}

#[test]
fn batch47_rhystic_nonpayable_players_cannot_block_production() {
    let mut e = prepared(&[], &[index::RHYSTIC_CAVE], &[]);
    let cave = on_battlefield(&e, P0, index::RHYSTIC_CAVE).unwrap();
    set_pool(&mut e, P0, &[]);
    set_pool(&mut e, P1, &[]);
    activate_as(&mut e, P0, cave, 0);
    e.apply(P0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    while let Pending::YesNo { player, .. } = e.pending().clone() {
        // An unaffordable attempted payment follows the existing unpaid
        // branch rather than rejecting the answer; it must not stop production.
        e.apply(player, PlayerAction::YesNo(true)).unwrap();
    }
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::Blue), 1);
}

#[test]
fn batch47_reflecting_pool_sees_rhystic_caves_possible_colors() {
    let mut e = prepared(&[], &[index::RHYSTIC_CAVE, index::REFLECTING_POOL], &[]);
    let pool = on_battlefield(&e, P0, index::REFLECTING_POOL).unwrap();
    set_pool(&mut e, P0, &[]);
    activate_as(&mut e, P0, pool, 0);
    let Pending::ChooseColor { options, .. } = e.pending() else {
        panic!("pool color choice")
    };
    assert_eq!(options.len(), 5);
    assert!(!options.contains(&ManaColor::Colorless));
    e.apply(P0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::Red), 1);
}
