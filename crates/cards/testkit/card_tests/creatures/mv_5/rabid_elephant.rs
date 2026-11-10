//! Oracle scenarios for Rabid Elephant.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn rabid_elephant() -> CardIndex {
    card_index("9daed342-e773-43ec-a8e2-98126b505e8a")
}

#[test]
fn rabid_elephant_gets_plus_two_plus_two_for_each_creature_blocking_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::FOREST; 5])
        .battlefield(1, &[index::GRIZZLY_BEARS, index::GRIZZLY_BEARS])
        .hand(0, &[rabid_elephant()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    cast_from_hand(&mut e, p0, rabid_elephant());
    pass_until(&mut e, stack_is_empty);

    let elephant = on_battlefield(&e, p0, rabid_elephant()).unwrap();
    let bears = all_on_battlefield(&e, p1, index::GRIZZLY_BEARS);
    assert_eq!(pt(&e, elephant), (3, 4));

    reach_their_main_phase(&mut e, p1);
    assert!(walk_to_own_main(&mut e, p0));
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(elephant, Defender::Player(p1))],
        },
    )
    .unwrap();
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == p1),
    );
    e.apply(
        p1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(bears[0], elephant), (bears[1], elephant)],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert_eq!(pt(&e, elephant), (7, 8));
}
