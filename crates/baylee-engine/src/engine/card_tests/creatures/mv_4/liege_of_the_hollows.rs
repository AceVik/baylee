//! `cards/creatures/mv_4/liege_of_the_hollows.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::LIEGE_OF_THE_HOLLOWS;

/// `Liege of the Hollows` ({2}{G}{G}, Creature — Spirit 3/4, Partial) enters as
/// a 3/4 Spirit.
#[test]
fn liege_of_the_hollows_casts_as_3_4_spirit() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[LIEGE_OF_THE_HOLLOWS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, LIEGE_OF_THE_HOLLOWS);
    pass_until(&mut engine, stack_is_empty);

    let liege = on_battlefield(&engine, p0, LIEGE_OF_THE_HOLLOWS).unwrap();
    assert_eq!(pt(&engine, liege), (3, 4));
}
