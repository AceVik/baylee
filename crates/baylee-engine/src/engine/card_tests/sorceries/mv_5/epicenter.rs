//! Oracle scenarios for Epicenter.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn epicenter() -> CardIndex {
    card_index("264cb7f6-6d36-4e58-87f4-83ccaa998a32")
}

#[test]
fn epicenter_without_threshold_forces_target_player_to_sacrifice_one_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::MOUNTAIN)
        .battlefield(0, &[index::MOUNTAIN; 5])
        .battlefield(1, &[index::FOREST, index::FOREST])
        .hand(0, &[epicenter()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let p1_forests = all_on_battlefield(&e, p1, index::FOREST);
    assert_eq!(p1_forests.len(), 2);

    cast_from_hand(&mut e, p0, epicenter());
    e.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p1),
    );
    e.apply(
        p1,
        PlayerAction::ChooseObjects {
            objects: vec![p1_forests[0]],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert_eq!(all_on_battlefield(&e, p1, index::FOREST).len(), 1);
    assert_eq!(all_on_battlefield(&e, p0, index::MOUNTAIN).len(), 5);
}

#[test]
fn epicenter_with_threshold_makes_each_player_sacrifice_all_lands() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board = vec![index::MOUNTAIN; 5];
    board.extend_from_slice(&[index::PLAINS; 7]);
    let mut e = Duel::new(SEED, index::MOUNTAIN)
        .battlefield(0, &board)
        .battlefield(1, &[index::FOREST, index::FOREST, index::GRIZZLY_BEARS])
        .hand(0, &[epicenter()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let plains = all_on_battlefield(&e, p0, index::PLAINS);
    for &p in &plains {
        e.dev_state_mut(p0)
            .unwrap()
            .move_object(
                p,
                ZoneLocation::Graveyard(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    e.refresh_offer();

    cast_from_hand(&mut e, p0, epicenter());
    e.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(all_on_battlefield(&e, p0, index::MOUNTAIN).is_empty());
    assert!(all_on_battlefield(&e, p1, index::FOREST).is_empty());
    assert!(on_battlefield(&e, p1, index::GRIZZLY_BEARS).is_some());
}
