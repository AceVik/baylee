//! Oracle scenarios for Seton's Desire.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn seton_s_desire() -> CardIndex {
    card_index("76fe7cfa-c3db-4122-b262-f7c5c90fcdd7")
}

#[test]
fn seton_s_desire_pumps_and_forces_all_able_creatures_to_block_with_threshold() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board = vec![
        index::GRIZZLY_BEARS,
        index::FOREST,
        index::FOREST,
        index::FOREST,
    ];
    board.extend_from_slice(&[index::PLAINS; 7]);
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &board)
        .battlefield(1, &[index::HILL_GIANT])
        .hand(0, &[seton_s_desire()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&e, p1, index::HILL_GIANT).unwrap();
    let plains = all_on_battlefield(&e, p0, index::PLAINS);
    bury(&mut e, &plains);

    cast_from_hand(&mut e, p0, seton_s_desire());
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![bear],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert_eq!(pt(&e, bear), (4, 4));

    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bear, Defender::Player(p1))],
        },
    )
    .unwrap();
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == p1),
    );

    assert!(
        e.apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
            .is_err(),
        "with Threshold, Hill Giant must block enchanted creature if able"
    );
    e.apply(
        p1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(giant, bear)],
        },
    )
    .unwrap();
}
