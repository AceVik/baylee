//! `cards/creatures/mv_4/cunning_advisor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn cunning_advisor() -> CardIndex {
    card_index("b80b6dd3-320b-4304-8073-f74c54e9a8fe")
}

/// Cunning Advisor (b80b6dd3-320b-4304-8073-f74c54e9a8fe) — `{3}{B}` 1/1
/// Creature — Human Advisor:
/// "`{T}`: Target opponent discards a card. Activate only during your turn,
/// before attackers are declared."
#[test]
fn cunning_advisor_makes_target_opponent_discard_before_attackers_are_declared() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[cunning_advisor()])
        .hand(1, &[forest(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, cunning_advisor());
    pass_until(&mut engine, stack_is_empty);
    let advisor = on_battlefield(&engine, P0, cunning_advisor()).expect("Cunning Advisor entered");
    assert_eq!(pt(&engine, advisor), (1, 1));

    reach_their_main_phase(&mut engine, P1);
    assert!(
        !ability_offered(&engine, advisor, 0),
        "cannot activate during opponent's turn"
    );

    // P0's first main phase is before attackers are declared.
    reach_their_main_phase(&mut engine, P0);
    let keep = in_hand(&engine, P1, forest()).unwrap();
    let pitch = in_hand(&engine, P1, island()).unwrap();
    assert!(ability_offered(&engine, advisor, 0));

    activate(&mut engine, P0, cunning_advisor(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .expect("target opponent P1");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseCards, got {:?}", engine.pending());
    };
    assert_eq!(player, P1);
    assert_eq!((min, max), (1, 1));
    assert!(options.contains(&keep) && options.contains(&pitch));

    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![pitch],
            },
        )
        .expect("P1 chooses Island to discard");
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P1, island()).is_some());
    assert!(in_hand(&engine, P1, forest()).is_some());
}

#[test]
fn cunning_advisor_cannot_activate_at_or_after_declare_attackers() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[cunning_advisor()])
        .hand(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    let advisor = on_battlefield(&engine, P0, cunning_advisor()).unwrap();

    // Upkeep is before DeclareAttackers on P0's turn -> allowed.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player: P0, .. })
    });
    assert!(ability_offered(&engine, advisor, 0));

    // DeclareAttackers step and SecondMain are not before attackers are declared -> forbidden.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(P0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();
    assert!(
        !ability_offered(&engine, advisor, 0),
        "cannot activate during DeclareAttackers step after attackers are declared"
    );

    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(
        !ability_offered(&engine, advisor, 0),
        "cannot activate during second main phase"
    );
}
