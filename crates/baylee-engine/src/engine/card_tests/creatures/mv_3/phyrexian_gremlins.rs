//! `cards/creatures/mv_3/phyrexian_gremlins.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Phyrexian Gremlins — {2}{B} 1/1: "You may choose not to untap this
/// creature during your untap step. {T}: Tap target artifact. It doesn't untap
/// during its controller's untap step for as long as this creature remains
/// tapped."
fn phyrexian_gremlins() -> CardIndex {
    card_index("407a0761-7ccc-4607-8df6-e744d30a81a0")
}

/// Turn one, main phase: Gremlins on my side, a Sol Ring on theirs. The
/// Gremlins tap the Ring and it resolves.
fn locked() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[phyrexian_gremlins()])
        .battlefield(1, &[sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let gremlins = on_battlefield(&engine, p0, phyrexian_gremlins()).expect("Gremlins");
    let ring = on_battlefield(&engine, p1, sol_ring()).expect("their Ring");
    assert!(!is_tapped(&engine, ring));
    activate(&mut engine, p0, phyrexian_gremlins(), 1);
    let options = aim_at(&mut engine, p0, ring);
    assert!(options.contains(&ring));
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, gremlins), "{{T}} was paid");
    assert!(is_tapped(&engine, ring), "the Ring was tapped");
    (engine, gremlins, ring)
}

/// Walks until the untap step asks which permanents stay tapped, handing the
/// question back unanswered (`answer_one` would untap).
#[track_caller]
fn to_the_untap_question(engine: &mut Engine<RegistryLookup>) -> Vec<ObjectId> {
    for _ in 0..200 {
        if let Pending::ChooseCards {
            options,
            prompt: ChoicePrompt::LeaveTapped,
            ..
        } = engine.pending().clone()
        {
            return options;
        }
        let (player, action) = answer_one(engine).expect("a rest on the way");
        engine.apply(player, action).expect("legal");
    }
    panic!("the untap step never asked");
}

/// The Ring does not untap in its controller's untap step while the Gremlins
/// stay tapped.
#[test]
fn phyrexian_gremlins_keep_the_artifact_down_through_its_untap_step() {
    let (mut engine, gremlins, ring) = locked();
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert!(is_tapped(&engine, gremlins));
    assert!(
        is_tapped(&engine, ring),
        "the lock held in their untap step"
    );
}

/// Once the Gremlins untap (my untap step), the Ring untaps normally in its
/// controller's next untap step.
#[test]
fn phyrexian_gremlins_release_the_artifact_once_they_untap() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = locked().0;
    let ring = on_battlefield(&engine, p1, sol_ring()).expect("Ring");
    let gremlins = on_battlefield(&engine, p0, phyrexian_gremlins()).expect("Gremlins");
    reach_their_main_phase(&mut engine, p1);
    let options = to_the_untap_question(&mut engine);
    assert!(options.contains(&gremlins));
    let (player, action) = answer_one(&engine).expect("the question");
    engine.apply(player, action).expect("legal");
    reach_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, gremlins), "untapped");
    assert!(
        is_tapped(&engine, ring),
        "the Ring itself untaps only on its own step"
    );
    reach_their_main_phase(&mut engine, p1);
    assert!(!is_tapped(&engine, ring), "released: untapped normally");
}

/// Choosing not to untap the Gremlins keeps the lock into their next untap.
#[test]
fn phyrexian_gremlins_keep_the_lock_while_i_choose_not_to_untap_them() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = locked().0;
    let ring = on_battlefield(&engine, p1, sol_ring()).expect("Ring");
    let gremlins = on_battlefield(&engine, p0, phyrexian_gremlins()).expect("Gremlins");
    reach_their_main_phase(&mut engine, p1);
    to_the_untap_question(&mut engine);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![gremlins],
            },
        )
        .expect("leave tapped");
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    assert!(is_tapped(&engine, gremlins));
    assert!(is_tapped(&engine, ring), "still locked two untap steps on");
}

/// The Gremlins dying releases the lock.
#[test]
fn phyrexian_gremlins_dying_releases_the_artifact() {
    let p1 = PlayerId::new(1);
    let (mut engine, gremlins, ring) = locked();
    kill(&mut engine, gremlins);
    assert!(in_graveyard(&engine, PlayerId::new(0), phyrexian_gremlins()).is_some());
    reach_their_main_phase(&mut engine, p1);
    assert!(!is_tapped(&engine, ring), "nothing holds it down any more");
}
