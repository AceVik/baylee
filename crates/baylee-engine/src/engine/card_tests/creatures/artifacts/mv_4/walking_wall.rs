//! Walking Wall — `{4}` 0/6 Artifact Creature — Wall (Ice Age)
//!
//! Defender
//! `{3}`: Walking Wall gets +3/-1 until end of turn and can attack this turn as
//! though it didn't have defender. Activate only once each turn.

#[allow(clippy::wildcard_imports)]
use super::*;

fn walking_wall() -> CardIndex {
    card_index("909201e5-5599-442a-9e4f-ff61f187e957")
}

#[test]
fn walking_wall_cannot_attack_until_activated_and_activates_only_once_each_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                walking_wall(),
                grizzly_bears(), // can attack on any turn: the contrast for defender
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wall = on_battlefield(&engine, p0, walking_wall()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    assert_eq!(pt(&engine, wall), (0, 6));

    // Activate {3} once: Walking Wall becomes 3/5 and can attack this turn.
    tap_all_mana(&mut engine, p0);
    assert!(ability_offered(&engine, wall, 0));
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: wall,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, wall), (3, 5));
    assert!(
        !ability_offered(&engine, wall, 0),
        "Walking Wall's ability can be activated only once each turn even with {{3}} floating"
    );

    // Attack p1 for 3 combat damage.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(wall, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 17);

    // On p0's next turn without activating the ability, Walking Wall is 0/6 and has defender.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(pt(&engine, wall), (0, 6));
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        panic!("expected ChooseAttackers");
    };
    assert!(
        attackers.contains(&bears),
        "the menu is open: Grizzly Bears may attack"
    );
    assert!(
        !attackers.contains(&wall),
        "without activating its ability, Walking Wall has defender and cannot attack"
    );
}
