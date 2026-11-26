//! `cards/creatures/mv_4/luminous_guardian.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn luminous_guardian() -> CardIndex {
    card_index("0d9a31f5-20d0-4266-95ec-38b4fa96b8aa")
}

/// Luminous Guardian activates {W} for +0/+1 until end of turn and {2} to block
/// an additional creature this turn, letting it block two 2/2 Bears and survive.
#[test]
fn luminous_guardian_pumps_toughness_and_blocks_two_attackers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[luminous_guardian(), plains(), plains(), plains()])
        .battlefield(1, &[grizzly_bears(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let guardian = on_battlefield(&engine, p0, luminous_guardian()).unwrap();
    let bears = all_on_battlefield(&engine, p1, grizzly_bears());

    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    let my_plains = all_on_battlefield(&engine, p0, plains());
    tap_mana_where(&mut engine, p0, |id| id == my_plains[0]);
    activate(&mut engine, p0, luminous_guardian(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, guardian), (1, 5));

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_mana_where(&mut engine, p0, |id| my_plains[1..3].contains(&id));
    activate(&mut engine, p0, luminous_guardian(), 1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (bears[0], Defender::Player(p0)),
                    (bears[1], Defender::Player(p0)),
                ],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(guardian, bears[0]), (guardian, bears[1])],
            },
        )
        .expect("blocks both attackers");
    loop {
        if engine.state().turn.phase == Phase::SecondMain && stack_is_empty(&engine) {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseNumber { player, max, .. } => {
                engine
                    .apply(player, PlayerAction::ChooseNumber(max))
                    .unwrap();
            }
            other => panic!("unexpected combat pending: {other:?}"),
        }
    }

    assert_eq!(lk::life(&engine, p0), 20);
    assert!(on_battlefield(&engine, p0, luminous_guardian()).is_some());
    assert_eq!(engine.state().object(guardian).unwrap().damage, 4);
}
