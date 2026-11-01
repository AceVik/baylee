//! `cards/creatures/mv_8/avatar_of_fury.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn avatar_of_fury() -> CardIndex {
    card_index("4854f836-2247-43c3-adbc-1d1ee5eaf120")
}

#[test]
fn avatar_of_fury_casts_flies_and_pumps_power_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[avatar_of_fury()])
        .battlefield(0, &[mountain(); 10])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, avatar_of_fury());
    pass_until(&mut engine, stack_is_empty);

    let avatar = on_battlefield(&engine, p0, avatar_of_fury()).unwrap();
    assert_eq!(pt(&engine, avatar), (6, 6));

    // Activate `{R}: +1/+0 until end of turn` twice using the two remaining Mountains.
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, avatar_of_fury(), 0);
    pass_until(&mut engine, stack_is_empty);
    activate(&mut engine, p0, avatar_of_fury(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, avatar), (8, 6));

    // Next turn, attack with Avatar of Fury; non-flying Grizzly Bears cannot block.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(pt(&engine, avatar), (6, 6));

    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(avatar, Defender::Player(p1))],
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
                    blockers: vec![(bears, avatar)],
                },
            )
            .is_err()
    );
}
