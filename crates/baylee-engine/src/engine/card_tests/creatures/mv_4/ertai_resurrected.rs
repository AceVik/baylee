//! `cards/creatures/mv_4/ertai_resurrected.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ertai Resurrected's second mode, which is the mutant for the collection
/// arm: it is the only place in the pool where
/// `Effect::DrawCardsFor { who: PlayerRel::ControllerOfTarget }` can run at
/// all, and a modal trigger that never fires is a card whose whole printed
/// text is unreachable. "Destroy another target creature or planeswalker.
/// Its controller draws a card."
#[test]
fn ertais_chosen_mode_destroys_and_lets_its_victim_draw() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(71, island())
        .battlefield(0, &[island(), island(), swamp(), swamp()])
        .battlefield(1, &[quiet_creature()])
        .hand(0, &[ertai_resurrected()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elves = on_battlefield(&engine, p1, quiet_creature()).expect("the opponent's creature");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();
    cast_from_hand(&mut engine, p0, ertai_resurrected());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    // Answered by *position*, and the position is not the mode number here:
    // with an empty stack Ertai's first mode has no spell or ability to
    // counter, so CR 603.3c takes it off the list and "destroy" is offered
    // first. A test that sent `ChooseMode(1)` picked the decline instead —
    // which is what it did before this line existed, and it failed loudly
    // rather than quietly, because the destroy asked for no target.
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        unreachable!("just checked")
    };
    let modes: Vec<usize> = options
        .iter()
        .filter_map(|o| match o.kind {
            CastModeKind::Mode(m) => Some(m),
            _ => None,
        })
        .collect();
    assert_eq!(
        modes,
        vec![1, 2],
        "destroy and decline; \"counter target spell, activated ability, or \
         triggered ability\" has nothing on an empty stack",
    );
    let destroy = modes
        .iter()
        .position(|m| *m == 1)
        .expect("the destroy mode is offered");
    engine.apply(p0, PlayerAction::ChooseMode(destroy)).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "the destroy mode asked for no target — got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&elves),
        "another creature is a legal target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, quiet_creature()).is_some(),
        "the targeted creature was destroyed",
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        hand_before + 1,
        "\"its controller draws a card\" — the *target's* controller, not \
         Ertai's; `PlayerRel::ControllerOfTarget` has never resolved for a \
         trigger before, because no modal trigger ever reached the stack",
    );
}
