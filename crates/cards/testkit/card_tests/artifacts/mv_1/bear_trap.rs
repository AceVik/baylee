//! Oracle checklist: Flash permits casting during the opponent's turn; activated cost is {3}, tap, sacrifice; target creature; exactly 3 damage.
#[allow(clippy::wildcard_imports)]
use super::*;
fn bear_trap() -> CardIndex {
    card_index("f7afb2c3-50ee-40f5-8b14-32f80c2bacd0")
}
#[test]
fn flash_trap_can_be_cast_on_the_opponents_turn_then_damages_a_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), gray_ogre()])
        .hand(0, &[bear_trap()])
        .battlefield(1, &[earth_elemental(), sol_ring()])
        .start();
    keep_mulligans(&mut e);
    reach_their_main_phase(&mut e, p1);
    pass_until(
        &mut e,
        |e| matches!(e.pending(),Pending::Priority{player,..} if *player==p0),
    );
    cast_from_hand(&mut e, p0, bear_trap());
    pass_until(&mut e, stack_is_empty);
    let trap = on_battlefield(&e, p0, bear_trap()).expect("flash spell resolved off-turn");
    pass_until(
        &mut e,
        |e| matches!(e.pending(),Pending::Priority{player,..} if *player==p0),
    );
    let target = on_battlefield(&e, p1, earth_elemental()).unwrap();
    let own = on_battlefield(&e, p0, gray_ogre()).unwrap();
    let artifact = on_battlefield(&e, p1, sol_ring()).unwrap();
    tap_all_mana(&mut e, p0);
    assert_eq!(
        e.state().players[0]
            .mana_pool
            .available(baylee_core::mana::ManaColor::Green),
        3,
        "the spell's {{1}} left three lands available"
    );
    activate(&mut e, p0, bear_trap(), 0);
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = e.pending()
    else {
        panic!("the activated ability asks for a creature target")
    };
    assert!(options.contains(&target));
    assert!(options.contains(&own));
    assert!(!options.contains(&artifact));
    assert!(player_options.is_empty());
    aim_at(&mut e, p0, target);
    assert_eq!(
        e.state().players[0]
            .mana_pool
            .available(baylee_core::mana::ManaColor::Green),
        0,
        "the {{3}} activation consumes all three available mana"
    );
    assert!(
        in_graveyard(&e, p0, bear_trap()).is_some(),
        "sacrifice precedes resolution"
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(target).unwrap().damage, 3);
    assert_eq!(e.state().object(own).unwrap().damage, 0);
    assert!(
        e.state()
            .object(trap)
            .is_some_and(|o| o.zone == Zone::Graveyard)
    );
}

#[test]
fn insufficient_mana_and_a_tapped_trap_are_not_offered() {
    let p0 = PlayerId::new(0);
    for tapped in [false, true] {
        let mut board = vec![bear_trap()];
        board.extend(vec![forest(); if tapped { 3 } else { 2 }]);
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &board)
            .battlefield(1, &[gray_ogre()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let source = on_battlefield(&e, p0, bear_trap()).unwrap();
        if tapped {
            e.dev_state_mut(p0)
                .unwrap()
                .object_mut(source)
                .unwrap()
                .status
                .insert(Status::TAPPED);
            e.refresh_offer();
        }
        assert_eq!(tap_all_mana(&mut e, p0), if tapped { 3 } else { 2 });
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("the player has priority")
        };
        assert!(!legal.abilities.iter().any(|(id, _)| *id == source));
        assert!(on_battlefield(&e, p0, bear_trap()).is_some());
    }
}
