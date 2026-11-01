#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn opal_avenger() -> CardIndex {
    card_index("13a85c9f-f653-4d90-bea6-94022ee82527")
}

#[test]
fn opal_avenger_becomes_3_5_soldier_when_life_at_most_ten() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[mountain(), opal_avenger()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let avenger = on_battlefield(&engine, p0, opal_avenger()).unwrap();
    engine.dev_state_mut(p0).unwrap().players[0].life = 13;

    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(avenger)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
    assert_eq!(pt(&engine, avenger), (3, 5));
}

#[test]
fn opal_avenger_ignores_an_opponents_low_life_and_eleven_is_too_high() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[opal_avenger()])
        .life(0, 11)
        .life(1, 10)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let avenger = lk::obj(&engine, p0, opal_avenger());
    assert!(
        engine
            .state()
            .object(avenger)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::ENCHANTMENT)
    );
    assert!(
        !engine
            .state()
            .object(avenger)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
}
