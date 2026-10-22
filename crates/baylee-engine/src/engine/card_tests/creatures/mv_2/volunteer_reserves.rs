//! `cards/creatures/mv_2/volunteer_reserves.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
fn volunteer_reserves() -> CardIndex {
    card_index("a2dabd52-0b75-42d7-a07d-0df7b38646e2")
}

/// `Volunteer Reserves` ({1}{W}, 2/4 Human Soldier) has banding and cumulative
/// upkeep `{1}`.
#[test]
fn volunteer_reserves_has_banding_and_cumulative_upkeep() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[volunteer_reserves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, volunteer_reserves());
    pass_until(&mut engine, stack_is_empty);

    let reserves = on_battlefield(&engine, p0, volunteer_reserves()).expect("Reserves");
    assert_eq!(pt(&engine, reserves), (2, 4));
    assert!(
        engine
            .state()
            .object(reserves)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::BANDING)
    );

    // Advance to p0's next upkeep and decline the cumulative upkeep {1} payment.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p0),
    );
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, volunteer_reserves()).is_some());
}
