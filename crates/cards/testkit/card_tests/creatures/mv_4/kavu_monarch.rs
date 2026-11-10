//! `cards/creatures/mv_4/kavu_monarch.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Kavu Monarch` is a 3/3 creature costing `{2}{R}{R}` under `Coverage::Implemented`.
/// It prints "Kavu creatures have trample." and "Whenever another Kavu enters, put a +1/+1 counter on this creature."
/// While on the battlefield, both it and another Kavu have trample, and casting another Kavu (such as `Raging Kavu`)
/// triggers its arrival ability, placing a +1/+1 counter on `Kavu Monarch`.
#[test]
fn kavu_monarch_grants_trample_to_kavus_and_grows_when_another_kavu_enters() {
    let p0 = PlayerId::new(0);
    let raging_kavu_card = card_index("d126bb08-be60-4046-91f1-65672ef42c63");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[kavu_monarch(), mountain(), forest(), mountain()])
        .hand(0, &[raging_kavu_card])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let monarch =
        on_battlefield(&engine, p0, kavu_monarch()).expect("Kavu Monarch is on the battlefield");
    assert_eq!(pt(&engine, monarch), (3, 3), "base body is 3/3");
    assert!(
        keywords(&engine, monarch).contains(KeywordSet::TRAMPLE),
        "Kavu Monarch grants trample to itself"
    );
    assert_eq!(
        counters_on(&engine, monarch, CounterKind::P1P1),
        0,
        "starts with zero counters"
    );

    cast_from_hand(&mut engine, p0, raging_kavu_card);
    pass_until(&mut engine, stack_is_empty);

    let raging =
        on_battlefield(&engine, p0, raging_kavu_card).expect("Raging Kavu is on the battlefield");
    assert!(
        keywords(&engine, raging).contains(KeywordSet::TRAMPLE),
        "static ability grants trample to the newcomer Kavu"
    );
    assert_eq!(
        counters_on(&engine, monarch, CounterKind::P1P1),
        1,
        "arrival trigger placed a +1/+1 counter on Kavu Monarch"
    );
    assert_eq!(pt(&engine, monarch), (4, 4), "body is now 4/4");
}
