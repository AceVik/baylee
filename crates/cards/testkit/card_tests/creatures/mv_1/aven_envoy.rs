//! `cards/creatures/mv_1/aven_envoy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Aven Envoy` prints `Flying` on a 0/2 creature with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Aven Envoy` while seat 1 attacks with `llanowar_elves()`.
/// As a flying creature, `Aven Envoy` can block ground attackers, and its 0/2 body absorbs 1 damage without dying or dealing counter damage.
#[test]
fn aven_envoy_has_flying_and_blocks_with_zero_two_body() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[aven_envoy()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);

    let bird = on_battlefield(&engine, p0, aven_envoy()).expect("aven envoy seated");
    assert_eq!(pt(&engine, bird), (0, 2));
    assert!(
        keywords(&engine, bird).contains(KeywordSet::FLYING),
        "`Aven Envoy` has flying"
    );

    reach_their_main_phase(&mut engine, p1);
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("elf seated");

    // Advance to declare attackers.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, Defender::Player(p0))],
            },
        )
        .expect("elf attacks p0");

    // Declare blockers.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on ChooseBlockers");
    };

    let option = blockers
        .iter()
        .find(|b| b.blocker == bird)
        .expect("aven envoy is offered as blocker");
    assert!(option.attackers.contains(&elf));

    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bird, elf)],
            },
        )
        .expect("aven envoy blocks elf");

    // Past the combat damage step, which an empty stack is not: the stack is
    // already empty the moment blockers are declared (CR 509.1), so waiting
    // for it returns before a point has been dealt.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    // Both survive: 0 power deals no damage to 1-toughness elf; 1 damage does not kill 2-toughness bird.
    assert!(
        on_battlefield(&engine, p0, aven_envoy()).is_some(),
        "aven envoy survives taking 1 damage"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "elf survives taking 0 damage"
    );
}
