//! `cards/creatures/mv_2/sneaky_homunculus.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn sneaky_homunculus_cant_block_or_be_blocked_by_power_2_or_greater() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[card_index("0b76bb3b-1ada-4c78-bd13-3464371d4fa0")])
        .battlefield(0, &[island(); 2])
        .battlefield(1, &[index::GRIZZLY_BEARS, llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(
        &mut engine,
        p0,
        card_index("0b76bb3b-1ada-4c78-bd13-3464371d4fa0"),
    );
    pass_until(&mut engine, stack_is_empty);

    let homunculus = on_battlefield(
        &engine,
        p0,
        card_index("0b76bb3b-1ada-4c78-bd13-3464371d4fa0"),
    )
    .unwrap();
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    let elves = on_battlefield(&engine, p1, llanowar_elves()).unwrap();

    // On p1's turn, Grizzly Bears (power 2) attacks; Sneaky Homunculus cannot block it.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(homunculus, bears)],
                },
            )
            .is_err()
    );
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    // On p0's next turn, Sneaky Homunculus attacks; Grizzly Bears (power 2) cannot block it, Llanowar Elves (power 1) can.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(homunculus, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(bears, homunculus)],
                },
            )
            .is_err()
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elves, homunculus)],
            },
        )
        .unwrap();
}
