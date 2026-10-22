//! `cards/creatures/mv_4/stalking_tiger.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn stalking_tiger() -> CardIndex {
    card_index("e6af1429-276f-41a8-88d0-062661dc0cd4")
}

/// Stalking Tiger (`{3}{G}`, `Coverage::Partial`): 3/3 Cat creature.
#[test]
fn stalking_tiger_enters_as_3_3_cat() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 4])
        .hand(0, &[stalking_tiger()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, stalking_tiger());
    pass_until(&mut engine, stack_is_empty);

    let tiger = on_battlefield(&engine, p0, stalking_tiger()).unwrap();
    assert_eq!(pt(&engine, tiger), (3, 3));
}
