//! Oracle scenarios for Mystic Crusader.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn mystic_crusader() -> CardIndex {
    card_index("937d663d-c9a7-4312-a4cc-fcb54faa92bc")
}

#[test]
fn mystic_crusader_has_protection_and_gains_stats_and_flying_with_threshold() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board = vec![index::PLAINS; 3];
    board.extend_from_slice(&[index::FOREST; 7]);
    board.push(index::GRIZZLY_BEARS);
    let mut e = Duel::new(SEED, index::PLAINS)
        .battlefield(0, &board)
        .battlefield(1, &[index::MOUNTAIN])
        .hand(0, &[mystic_crusader()])
        .hand(1, &[index::SHOCK])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let forests = all_on_battlefield(&e, p0, index::FOREST);
    for &f in &forests[..6] {
        e.dev_state_mut(p0)
            .unwrap()
            .move_object(
                f,
                ZoneLocation::Graveyard(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    e.refresh_offer();

    cast_from_hand(&mut e, p0, mystic_crusader());
    pass_until(&mut e, stack_is_empty);

    let id = on_battlefield(&e, p0, mystic_crusader()).unwrap();
    let bears = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    assert_eq!(pt(&e, id), (2, 1));
    assert!(!keywords(&e, id).contains(KeywordSet::FLYING));

    e.dev_state_mut(p0)
        .unwrap()
        .move_object(
            forests[6],
            ZoneLocation::Graveyard(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.apply(p0, PlayerAction::PassPriority).unwrap();

    assert_eq!(pt(&e, id), (3, 2));
    assert!(keywords(&e, id).contains(KeywordSet::FLYING));

    cast_from_hand(&mut e, p1, index::SHOCK);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected target choice")
    };
    assert!(!options.contains(&id), "protection from red blocks Shock");
    assert!(options.contains(&bears));
}

#[test]
fn mystic_crusader_protection_from_black_excludes_black_spells() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::PLAINS)
        .battlefield(0, &[mystic_crusader()])
        .battlefield(1, &[index::SWAMP, index::GRIZZLY_BEARS])
        .hand(1, &[index::DISFIGURE])
        .start();
    keep_mulligans(&mut e);
    reach_their_main_phase(&mut e, p1);

    let crusader = on_battlefield(&e, p0, mystic_crusader()).unwrap();
    let bears = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut e, p1, index::DISFIGURE);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected Disfigure targets")
    };
    assert!(!options.contains(&crusader));
    assert!(options.contains(&bears));
}
