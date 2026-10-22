//! `cards/creatures/mv_4/uktabi_efreet.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
fn uktabi_efreet() -> CardIndex {
    card_index("2fd9a37f-151a-4501-9d8e-bc67cc902592")
}

/// `Uktabi Efreet` ({2}{G}{G}, 5/4 Efreet) has cumulative upkeep `{G}`.
#[test]
fn uktabi_efreet_has_five_four_body_and_cumulative_upkeep_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[uktabi_efreet()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, uktabi_efreet());
    pass_until(&mut engine, stack_is_empty);

    let efreet = on_battlefield(&engine, p0, uktabi_efreet()).expect("Uktabi Efreet");
    assert_eq!(pt(&engine, efreet), (5, 4));

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p0),
    );
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, uktabi_efreet()).is_some());
}
