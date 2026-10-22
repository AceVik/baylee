//! `cards/creatures/artifacts/mv_4/crystal_golem.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::CRYSTAL_GOLEM;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Crystal Golem (82246ef2-4267-4daf-9463-908c2e142e7d) — {4} 3/3 Artifact Creature:
/// "At the beginning of your end step, this creature phases out."
#[test]
fn crystal_golem_phases_out_at_beginning_of_your_end_step_and_returns_next_turn() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[CRYSTAL_GOLEM])
        .start();
    keep_mulligans(&mut engine);

    let golem = on_battlefield(&engine, P0, CRYSTAL_GOLEM).unwrap();

    // Advance into P1's turn: Crystal Golem phased out at P0's end step.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        engine
            .state()
            .object(golem)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );

    // Advance to P0's next FirstMain: Crystal Golem phases back in during untap.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.number == 3
            && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        !engine
            .state()
            .object(golem)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
}
