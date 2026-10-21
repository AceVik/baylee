//! `cards/creatures/mv_3/mole_worms.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Phase;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mole_worms() -> CardIndex {
    card_index("b86ccfc1-a26b-4942-9d77-fef7676c77be")
}

/// Mole Worms (b86ccfc1-a26b-4942-9d77-fef7676c77be) — {2}{B} 1/1 Creature — Worm:
/// - "You may choose not to untap this creature during your untap step."
/// - "{T}: Tap target land. It doesn't untap during its controller's untap step for as long as this creature remains tapped."
#[test]
fn mole_worms_taps_target_land_and_keeps_it_tapped_only_while_mole_worms_stays_tapped() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[mole_worms()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let worms = on_battlefield(&engine, P0, mole_worms()).unwrap();
    let p1_land = on_battlefield(&engine, P1, forest()).unwrap();

    activate(&mut engine, P0, mole_worms(), 1);
    aim_at(&mut engine, P0, p1_land);
    pass_until(&mut engine, stack_is_empty);

    assert!(entered_tapped(&engine, worms));
    assert!(
        entered_tapped(&engine, p1_land),
        "Mole Worms taps the target land"
    );

    // Advance into P1's first main phase (turn 2): P1's land does not untap during P1's untap step.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 2
            && e.state().turn.active == P1
            && e.state().turn.phase == Phase::FirstMain
            && stack_is_empty(e)
    });
    assert!(
        entered_tapped(&engine, p1_land),
        "target land stays tapped through P1's untap step while Mole Worms is tapped"
    );

    // Advance to P0's untap step (turn 3): P0 is asked ChoicePrompt::LeaveTapped for Mole Worms.
    // Branch 1: P0 chooses to leave Mole Worms tapped.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::LeaveTapped,
                ..
            }
        )
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![worms],
            },
        )
        .expect("P0 may choose not to untap Mole Worms");
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3
            && e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && stack_is_empty(e)
    });
    assert!(
        entered_tapped(&engine, worms),
        "Mole Worms remained tapped because P0 chose LeaveTapped"
    );

    // Advance to P0's next untap step (turn 5): Branch 2 — P0 chooses to untap Mole Worms (empty LeaveTapped).
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::LeaveTapped,
                ..
            }
        )
    });
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .expect("P0 may choose to let Mole Worms untap");
    pass_until(&mut engine, |e| {
        e.state().turn.number == 5
            && e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && stack_is_empty(e)
    });
    assert!(
        !entered_tapped(&engine, worms),
        "Mole Worms untapped when P0 declined LeaveTapped"
    );

    // On P1's next turn (turn 6), P1's land now untaps during P1's untap step!
    pass_until(&mut engine, |e| {
        e.state().turn.number == 6
            && e.state().turn.active == P1
            && e.state().turn.phase == Phase::FirstMain
            && stack_is_empty(e)
    });
    assert!(
        !entered_tapped(&engine, p1_land),
        "P1's land untaps once Mole Worms is no longer tapped"
    );
}
