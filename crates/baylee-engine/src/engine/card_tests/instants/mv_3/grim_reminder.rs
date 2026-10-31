//! Grim Reminder reveals without moving, punishes once per opponent, and returns at upkeep.

use super::*;

fn grim_reminder() -> CardIndex {
    card_index("12684e95-2652-4c3c-b2c7-021b62344d77")
}

#[test]
fn grim_reminder_punishes_an_opponent_once_then_returns_from_graveyard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, gray_ogre())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .battlefield(1, &[mountain(); 8])
        .hand(0, &[grim_reminder()])
        .hand(1, &[gray_ogre(), gray_ogre()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    reach_their_main_phase(&mut e, p1);
    for _ in 0..2 {
        cast_from_hand(&mut e, p1, gray_ogre());
        pass_until(&mut e, stack_is_empty);
    }
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    let library_before = library_size(&e, p0);
    cast_from_hand(&mut e, p0, grim_reminder());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        panic!("expected the nonland search")
    };
    let chosen = options[0];
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![chosen],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().players[1].life,
        14,
        "one opponent loses six, not six per spell"
    );
    assert_eq!(
        library_size(&e, p0),
        library_before,
        "reveal and shuffle do not move the card out"
    );
    assert_eq!(e.state().object(chosen).unwrap().zone, Zone::Library);

    reach_their_main_phase(&mut e, p1);
    pass_until(&mut e, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { .. })
    });
    tap_all_mana(&mut e, p0);
    activate(&mut e, p0, grim_reminder(), 1);
    pass_until(&mut e, stack_is_empty);
    assert!(in_hand(&e, p0, grim_reminder()).is_some());
}
