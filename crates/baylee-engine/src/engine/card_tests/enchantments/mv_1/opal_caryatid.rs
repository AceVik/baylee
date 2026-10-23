#[allow(clippy::wildcard_imports)]
use super::*;

fn opal_caryatid() -> CardIndex {
    card_index("eabcb1bd-5317-4e01-8f18-94f01c12c852")
}

#[test]
fn opal_caryatid_becomes_2_2_soldier_when_opponent_casts_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[opal_caryatid()])
        .hand(1, &[grizzly_bears()])
        .battlefield(1, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p1));

    cast_from_hand(&mut engine, p1, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);

    let perm = on_battlefield(&engine, p0, opal_caryatid()).expect("seated");
    assert_eq!(pt(&engine, perm), (2, 2));
}
