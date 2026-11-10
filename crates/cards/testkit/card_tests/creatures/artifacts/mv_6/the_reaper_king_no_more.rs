//! `cards/creatures/artifacts/mv_6/the_reaper_king_no_more.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "A creature an opponent controls" is who controlled it as it died
/// (CR 603.10a), and the card in the graveyard is controlled by nobody. The
/// Guard, stolen with Treachery, dies as seat 0's creature and does not
/// trigger, though its owner is the opponent; the Giant dies as the
/// opponent's and does.
#[test]
fn the_reaper_asks_who_controlled_the_creature_as_it_died() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let treachery = card_index("8ed57194-7508-4aef-9373-64f7e80612d8");
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                the_reaper(),
                island(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .battlefield(1, &[steadfast_guard(), thundering_giant()])
        .hand(0, &[treachery])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let guard = on_battlefield(&engine, p1, steadfast_guard()).unwrap();
    let giant = on_battlefield(&engine, p1, thundering_giant()).unwrap();
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        for id in [guard, giant] {
            state
                .object_mut(id)
                .unwrap()
                .counters
                .add(CounterKind::M1M1, 1);
        }
    }
    cast_from_hand(&mut engine, p0, treachery);
    let _ = aim_at(&mut engine, p0, guard);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(guard).unwrap().controller, p0);

    reaper_sees_die(&mut engine, guard);
    assert!(!asked_may(&engine), "it was seat 0's creature as it died");
    assert!(in_graveyard(&engine, p1, steadfast_guard()).is_some());

    reaper_sees_die(&mut engine, giant);
    assert!(asked_may(&engine), "an opponent's, with a counter on it");
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, thundering_giant()).is_some());
}
