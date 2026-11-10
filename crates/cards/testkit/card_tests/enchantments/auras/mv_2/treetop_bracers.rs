//! `cards/enchantments/auras/mv_2/treetop_bracers.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn treetop_bracers_pumps_and_prevents_nonflying_blockers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[card_index("ba71eb8a-f9f8-4608-88d8-963a1770d8d4")])
        .battlefield(0, &[forest(), forest(), index::GRIZZLY_BEARS])
        .battlefield(1, &[llanowar_elves(), index::STORM_CROW])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).unwrap();
    let elves = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    let crow = on_battlefield(&engine, p1, index::STORM_CROW).unwrap();

    cast_from_hand(
        &mut engine,
        p0,
        card_index("ba71eb8a-f9f8-4608-88d8-963a1770d8d4"),
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bears), (3, 3));

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    // Nonflying Llanowar Elves cannot block.
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(elves, bears)],
                },
            )
            .is_err()
    );
    // Flying Storm Crow can block.
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(crow, bears)],
            },
        )
        .unwrap();
}
