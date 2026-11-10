//! Oracle scenarios for Burning Sands.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn burning_sands() -> CardIndex {
    card_index("262dd956-d90d-4782-91b6-6d183ec08b6c")
}

#[test]
fn burning_sands_forces_controller_of_dying_creature_to_sacrifice_a_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::MOUNTAIN)
        .battlefield(0, &[index::MOUNTAIN; 6])
        .battlefield(1, &[index::GRIZZLY_BEARS, index::FOREST])
        .hand(0, &[burning_sands(), index::SHOCK])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    cast_from_hand(&mut e, p0, burning_sands());
    pass_until(&mut e, stack_is_empty);

    let bears = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();
    let p1_forest = on_battlefield(&e, p1, index::FOREST).unwrap();

    cast_with_floating(&mut e, p0, index::SHOCK);
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![bears],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p1),
    );
    e.apply(
        p1,
        PlayerAction::ChooseObjects {
            objects: vec![p1_forest],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(in_graveyard(&e, p1, index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&e, p1, index::FOREST).is_some());
    assert_eq!(all_on_battlefield(&e, p0, index::MOUNTAIN).len(), 6);
}
