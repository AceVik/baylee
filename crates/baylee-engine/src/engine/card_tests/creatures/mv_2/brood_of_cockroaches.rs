//! `cards/creatures/mv_2/brood_of_cockroaches.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn brood_of_cockroaches() -> CardIndex {
    card_index("7a22d694-29d1-4e7b-8fca-d7008aede489")
}

#[test]
fn brood_of_cockroaches_dies_trigger_loses_life_and_returns_to_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[brood_of_cockroaches()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    cast_from_hand(&mut engine, p0, brood_of_cockroaches());
    pass_until(&mut engine, stack_is_empty);

    let brood = on_battlefield(&engine, p0, brood_of_cockroaches()).unwrap();
    assert_eq!(pt(&engine, brood), (1, 1));

    kill(&mut engine, brood);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 19);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
}
