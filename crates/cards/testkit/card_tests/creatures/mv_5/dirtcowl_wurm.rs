//! `cards/creatures/mv_5/dirtcowl_wurm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn dirtcowl_wurm() -> CardIndex {
    card_index("608293f1-34aa-4e55-8389-73f975858de6")
}

/// Opponent land drops put a +1/+1 counter on Dirtcowl Wurm, while its own
/// controller's land drops do not.
#[test]
fn dirtcowl_wurm_grows_on_opponents_land_drops_and_ignores_your_own() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 5])
        .hand(0, &[dirtcowl_wurm(), forest()])
        .hand(1, &[plains(), island()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, dirtcowl_wurm());
    pass_until(&mut engine, stack_is_empty);
    let wurm = on_battlefield(&engine, p0, dirtcowl_wurm()).expect("resolved");
    assert_eq!(pt(&engine, wurm), (3, 4));

    // Playing our own land does NOT trigger Dirtcowl Wurm.
    let own_land = in_hand(&engine, p0, forest()).unwrap();
    engine
        .apply(p0, PlayerAction::PlayLand { card: own_land })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, wurm), (3, 4), "own land play does not trigger");

    // Opponent's first land play puts a +1/+1 counter on Dirtcowl Wurm (4/5).
    reach_their_main_phase(&mut engine, p1);
    let opp_land1 = in_hand(&engine, p1, plains()).unwrap();
    engine
        .apply(p1, PlayerAction::PlayLand { card: opp_land1 })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, wurm), (4, 5));
    assert_eq!(
        engine
            .state()
            .object(wurm)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1
    );

    // Opponent's next turn land play adds a second +1/+1 counter (5/6).
    reach_their_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    let opp_land2 = in_hand(&engine, p1, island()).unwrap();
    engine
        .apply(p1, PlayerAction::PlayLand { card: opp_land2 })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, wurm), (5, 6));
    assert_eq!(
        engine
            .state()
            .object(wurm)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        2
    );
}
