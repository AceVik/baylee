//! Oracle scenarios for Mystic Enforcer.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn mystic_enforcer() -> CardIndex {
    card_index("cf3b79cc-ce97-48e9-a578-625e323d1b62")
}

#[test]
fn mystic_enforcer_gains_plus_three_plus_three_and_flying_with_threshold() {
    let p0 = PlayerId::new(0);
    let mut board = vec![index::FOREST, index::FOREST, index::PLAINS, index::PLAINS];
    board.extend_from_slice(&[index::ISLAND; 7]);
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &board)
        .hand(0, &[mystic_enforcer()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let islands = all_on_battlefield(&e, p0, index::ISLAND);
    for &i in &islands[..6] {
        e.dev_state_mut(p0)
            .unwrap()
            .move_object(
                i,
                ZoneLocation::Graveyard(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    e.refresh_offer();

    cast_from_hand(&mut e, p0, mystic_enforcer());
    pass_until(&mut e, stack_is_empty);

    let id = on_battlefield(&e, p0, mystic_enforcer()).unwrap();
    assert_eq!(pt(&e, id), (3, 3));
    assert!(!keywords(&e, id).contains(KeywordSet::FLYING));

    e.dev_state_mut(p0)
        .unwrap()
        .move_object(
            islands[6],
            ZoneLocation::Graveyard(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.apply(p0, PlayerAction::PassPriority).unwrap();

    assert_eq!(pt(&e, id), (6, 6));
    assert!(keywords(&e, id).contains(KeywordSet::FLYING));
}

#[test]
fn mystic_enforcer_protection_from_black_excludes_black_spells() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[mystic_enforcer()])
        .battlefield(1, &[index::SWAMP, index::GRIZZLY_BEARS])
        .hand(1, &[index::DISFIGURE])
        .start();
    keep_mulligans(&mut e);
    reach_their_main_phase(&mut e, p1);

    let enforcer = on_battlefield(&e, p0, mystic_enforcer()).unwrap();
    let bears = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut e, p1, index::DISFIGURE);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected Disfigure targets")
    };
    assert!(!options.contains(&enforcer));
    assert!(options.contains(&bears));
}
