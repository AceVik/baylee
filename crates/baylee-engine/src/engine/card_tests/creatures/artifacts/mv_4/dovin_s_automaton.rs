//! `cards/creatures/artifacts/mv_4/dovin_s_automaton.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn dovin_s_automaton() -> CardIndex {
    card_index("3d133b18-f1c7-48de-82a6-affb0514b96b")
}

/// Dovin's Automaton — {4} 3/3: "As long as you control a Dovin
/// planeswalker, this creature gets +2/+2 and has vigilance."
///
/// Only the branch without a Dovin can be played: the pool holds no Dovin
/// planeswalker card to put beside it, and a planeswalker is not put on a
/// development board (#304). Without one the Automaton is a plain 3/3 without
/// vigilance, so attacking taps it.
#[test]
fn dovin_s_automaton_is_a_plain_three_three_without_a_dovin() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dovin_s_automaton()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let automaton = on_battlefield(&engine, p0, dovin_s_automaton()).expect("seated");
    assert_eq!(pt(&engine, automaton), (3, 3));
    assert!(!keywords(&engine, automaton).contains(KeywordSet::VIGILANCE));

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(automaton, Defender::Player(p1))],
            },
        )
        .expect("it attacks");
    assert!(
        is_tapped(&engine, automaton),
        "no vigilance: attacking taps"
    );
}
