#[allow(clippy::wildcard_imports)]
use super::*;

fn multani_s_acolyte() -> CardIndex {
    card_index("d08725e6-ed9a-4c82-89d4-0dd037c376f6")
}

#[test]
fn multani_s_acolyte_draws_a_card_on_etb() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[multani_s_acolyte()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    cast_from_hand(&mut engine, p0, multani_s_acolyte());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, multani_s_acolyte()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
}
