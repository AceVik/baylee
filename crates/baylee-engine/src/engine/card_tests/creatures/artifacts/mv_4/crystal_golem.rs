//! `cards/creatures/artifacts/mv_4/crystal_golem.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn crystal_golem() -> CardIndex {
    card_index("82246ef2-4267-4daf-9463-908c2e142e7d")
}

/// Crystal Golem (82246ef2-4267-4daf-9463-908c2e142e7d) — {4} 3/3 Artifact Creature:
/// "At the beginning of your end step, this creature phases out."
#[test]
fn crystal_golem_phases_out_at_beginning_of_your_end_step_and_returns_next_turn() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[crystal_golem()])
        .start();
    keep_mulligans(&mut engine);

    let golem = on_battlefield(&engine, P0, crystal_golem()).unwrap();

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

/// "At the beginning of your end step": the Golem is still in play, not
/// phased out, in its controller's second main phase.
#[test]
fn crystal_golem_does_not_phase_out_before_the_end_step() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[crystal_golem()])
        .start();
    keep_mulligans(&mut engine);
    let golem = on_battlefield(&engine, P0, crystal_golem()).unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.phase == Phase::SecondMain
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

/// "your end step": the opponent's Golem is untouched by P0's end step, and
/// phases out at its own controller's.
#[test]
fn crystal_golem_phases_out_only_at_its_controllers_end_step() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(1, &[crystal_golem()])
        .start();
    keep_mulligans(&mut engine);
    let golem = on_battlefield(&engine, P1, crystal_golem()).unwrap();

    // P0's turn 1 is over: its end step did not phase P1's Golem out.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        !engine
            .state()
            .object(golem)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );

    // P1's own end step has passed by P0's next main phase.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.number == 3
            && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        engine
            .state()
            .object(golem)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
}
