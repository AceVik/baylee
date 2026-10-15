//! `cards/creatures/artifacts/mv_4/badgey_malicious_glitch.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn badgey_malicious_glitch() -> CardIndex {
    card_index("227fb13b-0a07-4937-9048-7cf02efbc8e3")
}

/// Badgey, Malicious Glitch — {3}{B} 4/3: "Whenever Badgey becomes tapped,
/// each opponent loses 1 life and you gain 1 life." Attacking taps it, so on
/// declaring it the trigger resolves before any damage: 19 against 21; the
/// unblocked 4 then take the opponent to 15.
#[test]
fn badgey_drains_one_when_it_becomes_tapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[badgey_malicious_glitch()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let badgey = on_battlefield(&engine, p0, badgey_malicious_glitch()).expect("seated");
    assert_eq!(pt(&engine, badgey), (4, 3));
    assert!(!is_tapped(&engine, badgey));

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(badgey, Defender::Player(p1))],
            },
        )
        .expect("Badgey attacks");
    assert!(is_tapped(&engine, badgey), "attacking tapped it");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 19, "each opponent loses 1");
    assert_eq!(engine.state().players[0].life, 21, "and you gain 1");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("no blocks");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 15, "the four went through");
    assert_eq!(
        engine.state().players[0].life,
        21,
        "and the trigger fired once"
    );
}
