//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn rack_sets_only_the_chosen_opponents_limit_and_later_tower_overrides_it() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut e = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[index::FOREST; 4])
        .hand(0, &[index::CURSED_RACK])
        .hand(
            2,
            &[
                index::RELIQUARY_TOWER,
                index::FOREST,
                index::FOREST,
                index::FOREST,
                index::FOREST,
                index::FOREST,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    cast_from_hand(&mut e, p0, index::CURSED_RACK);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. })
    });
    e.apply(p0, PlayerAction::ChoosePlayer(p2)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().maximum_hand_size(p0), Some(7));
    assert_eq!(e.state().maximum_hand_size(p1), Some(7));
    assert_eq!(e.state().maximum_hand_size(p2), Some(4));
    reach_their_main_phase(&mut e, p2);
    let tower = play_land(&mut e, p2, index::RELIQUARY_TOWER);
    assert_eq!(e.state().maximum_hand_size(p2), None);
    bury(&mut e, &[tower]);
    e.apply(p2, PlayerAction::PassPriority).unwrap();
    assert_eq!(e.state().maximum_hand_size(p2), Some(4));
    pass_until(
        &mut e,
        |e| matches!(e.pending(),Pending::DiscardChoice {player,..} if *player==p2),
    );
    let Pending::DiscardChoice { count, .. } = e.pending() else {
        unreachable!()
    };
    assert_eq!(
        usize::from(*count),
        e.state().zones.list(ZoneLocation::Hand(p2)).len() - 4
    );
}

#[test]
fn later_rack_overrides_an_existing_no_maximum_hand_size_effect() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 4])
        .battlefield(1, &[index::RELIQUARY_TOWER])
        .hand(0, &[index::CURSED_RACK])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    assert_eq!(e.state().maximum_hand_size(p1), None);
    cast_from_hand(&mut e, p0, index::CURSED_RACK);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. })
    });
    e.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().maximum_hand_size(p1), Some(4));
}
