//! `cards/creatures/mv_2/benevolent_unicorn.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::BENEVOLENT_UNICORN;

#[test]
fn benevolent_unicorn_enters_as_one_two_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[BENEVOLENT_UNICORN])
        .battlefield(0, &[plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, BENEVOLENT_UNICORN);
    pass_until(&mut engine, stack_is_empty);

    let unicorn = on_battlefield(&engine, p0, BENEVOLENT_UNICORN).unwrap();
    assert_eq!(pt(&engine, unicorn), (1, 2));
}
