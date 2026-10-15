//! `cards/creatures/artifacts/mv_2/voltaic_servant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn voltaic_servant() -> CardIndex {
    card_index("d0c2703b-bd74-41ac-b516-c46c654de1bb")
}

/// Voltaic Servant: a 1/3; "At the beginning of your end step, untap target
/// artifact." A Gold Myr tapped for mana in the main phase is untapped when the
/// end step comes, the Servant's trigger aimed at it.
#[test]
fn voltaic_servant_untaps_a_target_artifact_at_your_end_step() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[voltaic_servant(), gold_myr()], &[]);
    let servant = on_battlefield(&engine, p0, voltaic_servant()).expect("seated");
    assert_eq!(pt(&engine, servant), (1, 3));
    let myr = on_battlefield(&engine, p0, gold_myr()).expect("seated");
    activate(&mut engine, p0, gold_myr(), 0);
    assert!(is_tapped(&engine, myr));

    let options = pass_until_targets(&mut engine, p0);
    assert_eq!(engine.state().turn.step, Step::End, "at the end step");
    assert!(options.contains(&myr));
    assert!(options.contains(&servant), "the Servant is an artifact too");
    aim_at(&mut engine, p0, myr);
    pass_until(&mut engine, stack_is_empty);
    assert!(!is_tapped(&engine, myr), "untapped by the trigger");
}

/// The Servant's own trigger is on "your" end step only: the opponent's end
/// step asks nothing of seat 0.
#[test]
fn voltaic_servant_does_not_trigger_on_the_opponents_end_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(&[], &[voltaic_servant()], &[]);
    // Our own end step: the trigger asks for a target (the Servant itself).
    pass_until_targets(&mut engine, p0);
    let servant = on_battlefield(&engine, p0, voltaic_servant()).unwrap();
    aim_at(&mut engine, p0, servant);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::End
    });
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "the opponent's end step stays quiet: {:?}",
        engine.pending()
    );
}
