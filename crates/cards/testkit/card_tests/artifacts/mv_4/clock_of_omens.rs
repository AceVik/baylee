//! `cards/artifacts/mv_4/clock_of_omens.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn clock_of_omens() -> CardIndex {
    card_index("5d0f4665-aa10-4251-b590-129b59274fb3")
}

fn ornithopter() -> CardIndex {
    card_index("a3a98bc9-caa0-49b7-951c-fe4e4f54e4ba")
}

/// Drives Clock of Omens' activation: targets `target` and pays by tapping
/// `cost_taps`, returning both the target menu and the cost-tap menu.
#[track_caller]
fn activate_clock(
    engine: &mut Engine<RegistryLookup>,
    target: ObjectId,
    cost_taps: &[ObjectId],
) -> (Vec<ObjectId>, Vec<ObjectId>) {
    let p0 = PlayerId::new(0);
    activate(engine, p0, clock_of_omens(), 0);
    let mut target_menu = Vec::new();
    let mut tap_menu = Vec::new();
    for _ in 0..10 {
        if !target_menu.is_empty() && !tap_menu.is_empty() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player, options, ..
            } => {
                target_menu = options;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![target],
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(prompt, ChoicePrompt::CostTap);
                assert_eq!(
                    (min, max),
                    (2, 2),
                    "must tap exactly two untapped artifacts"
                );
                tap_menu = options;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: cost_taps.to_owned(),
                        },
                    )
                    .unwrap();
            }
            other => panic!("unexpected during Clock of Omens activation: {other:?}"),
        }
    }
    pass_until(engine, stack_is_empty);
    (target_menu, tap_menu)
}

/// Clock of Omens (5d0f4665-aa10-4251-b590-129b59274fb3) — {4} Artifact:
/// "Tap two untapped artifacts you control: Untap target artifact."
///
/// Can tap itself and another untapped artifact you control to untap a tapped
/// artifact (yours or opponent's), and is unactivatable with fewer than two
/// untapped artifacts under your control.
#[test]
fn clock_of_omens_taps_two_own_artifacts_to_untap_any_artifact() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[clock_of_omens(), sol_ring(), ornithopter(), forest()])
        .battlefield(1, &[sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let clock = on_battlefield(&engine, p0, clock_of_omens()).unwrap();
    let my_ring = on_battlefield(&engine, p0, sol_ring()).unwrap();
    let thopter = on_battlefield(&engine, p0, ornithopter()).unwrap();
    let my_forest = on_battlefield(&engine, p0, forest()).unwrap();
    let their_ring = on_battlefield(&engine, p1, sol_ring()).unwrap();

    // Tap own Sol Ring for mana: p0 now has 2 untapped artifacts (Clock, Ornithopter).
    tap_mana_where(&mut engine, p0, |id| id == my_ring);
    assert!(is_tapped(&engine, my_ring));

    // Activate Clock targeting own tapped Sol Ring, tapping Clock + Ornithopter.
    let (target_menu, tap_menu) = activate_clock(&mut engine, my_ring, &[clock, thopter]);
    assert!(
        target_menu.contains(&my_ring) && target_menu.contains(&their_ring),
        "target artifact includes both yours and opponent's: {target_menu:?}"
    );
    assert!(
        !target_menu.contains(&my_forest),
        "non-artifact Forest cannot be targeted: {target_menu:?}"
    );
    assert!(
        tap_menu.contains(&clock) && tap_menu.contains(&thopter),
        "both untapped artifacts you control are offered to pay: {tap_menu:?}"
    );
    assert!(
        !tap_menu.contains(&my_ring)
            && !tap_menu.contains(&their_ring)
            && !tap_menu.contains(&my_forest),
        "tapped artifact, opponent's artifact, and non-artifact excluded from cost: {tap_menu:?}"
    );
    assert!(is_tapped(&engine, clock) && is_tapped(&engine, thopter));
    assert!(!is_tapped(&engine, my_ring), "own Sol Ring was untapped");

    // Only 1 untapped artifact left under p0's control -> Clock cannot activate.
    assert!(
        !ability_offered(&engine, clock, 0),
        "needs two untapped artifacts you control (opponent's does not count)"
    );

    // Next turn: on p1's main phase, p1 taps their Sol Ring; p0 untaps p1's Sol Ring.
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    pass_until(&mut engine, |e| e.state().turn.active == p0);
    reach_their_main_phase(&mut engine, p1);
    tap_mana_where(&mut engine, p1, |id| id == their_ring);
    assert!(is_tapped(&engine, their_ring));

    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    activate_clock(&mut engine, their_ring, &[clock, thopter]);
    assert!(
        !is_tapped(&engine, their_ring),
        "untapped opponent's artifact"
    );
}
