//! `cards/creatures/mv_3/extraction_specialist.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "When this creature enters, return target creature card with mana value 2
/// or less from your graveyard to the battlefield. That creature can't attack
/// or block for as long as you control this creature." The Elves and the
/// Guard are offered and the Giant is not; the Elves return held back, and
/// the Specialist dying sets them free.
#[test]
fn extraction_specialist_returns_a_small_creature_that_cannot_attack_or_block() {
    let p0 = PlayerId::new(0);
    let (mut engine, options) = extraction_specialist_asks(&[], &[]);
    let elves = in_graveyard(&engine, p0, llanowar_elves()).unwrap();
    let guard = in_graveyard(&engine, p0, steadfast_guard()).unwrap();
    let giant = in_graveyard(&engine, p0, thundering_giant()).unwrap();
    assert!(options.contains(&elves) && options.contains(&guard));
    assert!(!options.contains(&giant), "mana value 7");
    let _ = aim_at(&mut engine, p0, elves);
    pass_until(&mut engine, stack_is_empty);

    let specialist = on_battlefield(&engine, p0, extraction_specialist()).unwrap();
    assert!(keywords(&engine, specialist).contains(KeywordSet::LIFELINK));
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("returned");
    assert!(held_back(&engine, elves), "can't attack or block");
    assert!(in_graveyard(&engine, p0, steadfast_guard()).is_some());

    kill(&mut engine, specialist);
    assert!(
        !held_back(&engine, elves),
        "for as long as you control this creature: no longer"
    );
}

/// A blink ends it: the Specialist that returns from Ephemerate is a new
/// object (CR 400.7), so the Elves it held are free, and its new trigger
/// holds the Guard instead.
#[test]
fn extraction_specialist_blinked_lets_the_first_creature_go() {
    let p0 = PlayerId::new(0);
    let (mut engine, _) = extraction_specialist_asks(&[ephemerate()], &[]);
    let elves = in_graveyard(&engine, p0, llanowar_elves()).unwrap();
    let _ = aim_at(&mut engine, p0, elves);
    pass_until(&mut engine, stack_is_empty);
    let specialist = on_battlefield(&engine, p0, extraction_specialist()).unwrap();
    let elves = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    assert!(held_back(&engine, elves));

    // The fourth Plains is still floating.
    cast_with_floating(&mut engine, p0, ephemerate());
    let _ = aim_at(&mut engine, p0, specialist);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    assert!(
        !held_back(&engine, elves),
        "its Specialist left the battlefield"
    );
    let guard = in_graveyard(&engine, p0, steadfast_guard()).unwrap();
    let _ = aim_at(&mut engine, p0, guard);
    pass_until(&mut engine, stack_is_empty);
    let guard = on_battlefield(&engine, p0, steadfast_guard()).unwrap();
    assert!(held_back(&engine, guard), "the new Specialist's creature");
    assert!(!held_back(&engine, elves));
}

/// CR 611.2b: a duration that is over before the effect begins never
/// starts. With the trigger on the stack, Ephemerate blinks the Specialist:
/// the one that returns is a new object (CR 400.7) and its own trigger holds
/// the Guard, while the first trigger still returns the Elves and holds
/// nothing — the Specialist it spoke of is gone.
#[test]
fn extraction_specialist_blinked_in_response_holds_nothing_with_its_first_trigger() {
    let p0 = PlayerId::new(0);
    let (mut engine, _) = extraction_specialist_asks(&[ephemerate()], &[]);
    let elves = in_graveyard(&engine, p0, llanowar_elves()).unwrap();
    let _ = aim_at(&mut engine, p0, elves);
    let specialist = on_battlefield(&engine, p0, extraction_specialist()).unwrap();
    cast_with_floating(&mut engine, p0, ephemerate());
    let _ = aim_at(&mut engine, p0, specialist);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let guard = in_graveyard(&engine, p0, steadfast_guard()).unwrap();
    let _ = aim_at(&mut engine, p0, guard);
    pass_until(&mut engine, stack_is_empty);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("returned all the same");
    let guard = on_battlefield(&engine, p0, steadfast_guard()).unwrap();
    assert!(!held_back(&engine, elves), "its Specialist was gone");
    assert!(held_back(&engine, guard), "the returned Specialist's");
}

/// The other way the duration ends: another player gains control of the
/// Specialist. The opponent's Treachery takes it, and the Elves its first
/// controller still controls are free.
#[test]
fn extraction_specialist_stolen_lets_the_creature_go() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let treachery = card_index("8ed57194-7508-4aef-9373-64f7e80612d8");
    let (mut engine, _) = extraction_specialist_asks(&[], &[treachery]);
    let elves = in_graveyard(&engine, p0, llanowar_elves()).unwrap();
    let _ = aim_at(&mut engine, p0, elves);
    pass_until(&mut engine, stack_is_empty);
    let specialist = on_battlefield(&engine, p0, extraction_specialist()).unwrap();
    let elves = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    assert!(held_back(&engine, elves));

    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, treachery);
    let _ = aim_at(&mut engine, p1, specialist);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(specialist).unwrap().controller, p1);
    assert_eq!(engine.state().object(elves).unwrap().controller, p0);
    assert!(!held_back(&engine, elves), "you no longer control it");
}

/// A third way it ends: the Specialist phases out. A phased-out permanent is
/// treated as though it does not exist (CR 702.26b), so you no longer
/// control it, and a "for as long as" duration that tracks it ends as it
/// phases out (CR 702.26f). The Elves are free while it is away and stay
/// free once it is back.
#[test]
fn extraction_specialist_phased_out_lets_the_creature_go() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let (mut engine, _) = extraction_specialist_asks(&[], &[]);
    let elves = in_graveyard(&engine, p0, llanowar_elves()).unwrap();
    let _ = aim_at(&mut engine, p0, elves);
    pass_until(&mut engine, stack_is_empty);
    let specialist = on_battlefield(&engine, p0, extraction_specialist()).unwrap();
    let elves = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    assert!(held_back(&engine, elves));

    phase_out_specialist(&mut engine, specialist);
    // One pass, so the engine runs the loop that ends durations.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert!(
        engine
            .state()
            .object(specialist)
            .is_some_and(|o| o.status.contains(crate::object::Status::PHASED_OUT)),
        "the Specialist phased out"
    );
    assert!(!held_back(&engine, elves), "it phased out");

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        engine
            .state()
            .object(specialist)
            .is_some_and(|o| !o.status.contains(crate::object::Status::PHASED_OUT)),
        "the Specialist phased in at seat 0's untap step"
    );
    assert!(
        !held_back(&engine, elves),
        "an ended duration does not begin again as the Specialist phases in"
    );
}

/// CR 611.2b again: the Specialist phases out with its trigger on the
/// stack, so "for as long as you control this creature" is over before the
/// effect would begin. The Elves return all the same and are free.
#[test]
fn extraction_specialist_phased_out_in_response_holds_nothing() {
    let p0 = PlayerId::new(0);
    let (mut engine, _) = extraction_specialist_asks(&[], &[]);
    let elves = in_graveyard(&engine, p0, llanowar_elves()).unwrap();
    let _ = aim_at(&mut engine, p0, elves);
    let specialist = on_battlefield(&engine, p0, extraction_specialist()).unwrap();
    phase_out_specialist(&mut engine, specialist);
    pass_until(&mut engine, stack_is_empty);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("returned all the same");
    assert!(!held_back(&engine, elves), "its Specialist was phased out");
}
