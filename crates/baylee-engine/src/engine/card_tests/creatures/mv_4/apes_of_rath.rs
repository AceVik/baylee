//! `cards/creatures/mv_4/apes_of_rath.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn apes_of_rath() -> CardIndex {
    card_index("b52f5850-f1aa-412a-bae9-2ce76c287188")
}

/// Apes of Rath (b52f5850-f1aa-412a-bae9-2ce76c287188) — {2}{G}{G} Creature — Ape:
/// "Whenever this creature attacks, it doesn't untap during its controller's next untap step."
#[test]
fn apes_of_rath_does_not_untap_during_next_untap_step_after_attacking() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[apes_of_rath()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let apes = on_battlefield(&engine, P0, apes_of_rath()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(apes, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(is_tapped(&engine, apes));
    assert_eq!(engine.state().players[1].life, 15);

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    assert!(
        is_tapped(&engine, apes),
        "Apes of Rath remains tapped across P0's next untap step"
    );
}
