#[allow(clippy::wildcard_imports)]
use super::*;

fn angel_s_trumpet() -> CardIndex {
    card_index("f187f766-3696-419e-9865-58023a46446b")
}

#[test]
fn angel_s_trumpet_grants_vigilance_and_taps_nonattacking_creatures_at_end_step() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[angel_s_trumpet()])
        .battlefield(0, &[gray_ogre(), mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, angel_s_trumpet());
    pass_until(&mut engine, stack_is_empty);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    assert!(
        !engine
            .state()
            .object(ogre)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );

    reach_their_main_phase(&mut engine, p1);
    assert!(
        engine
            .state()
            .object(ogre)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
}
