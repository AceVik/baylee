#[allow(clippy::wildcard_imports)]
use super::*;

fn acridian() -> CardIndex {
    card_index("c6d0f1bc-b82a-4e64-a4bc-11ad3f54c71a")
}

#[test]
fn acridian_cast_and_echo_trigger() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[acridian()])
        .battlefield(0, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, acridian());
    pass_until(&mut engine, stack_is_empty);
    let bug = on_battlefield(&engine, p0, acridian()).expect("seated");
    assert_eq!(pt(&engine, bug), (2, 4));

    // Advance through p0's next upkeep without paying Echo; Acridian is sacrificed.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(on_battlefield(&engine, p0, acridian()).is_none());
    assert!(in_graveyard(&engine, p0, acridian()).is_some());
}
