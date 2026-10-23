#[allow(clippy::wildcard_imports)]
use super::*;

fn opal_archangel() -> CardIndex {
    card_index("888d12d4-aaa5-406f-9dc8-8c4e856eb2fe")
}

#[test]
fn opal_archangel_becomes_5_5_angel_when_opponent_casts_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[opal_archangel()])
        .hand(1, &[grizzly_bears()])
        .battlefield(1, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p1));

    cast_from_hand(&mut engine, p1, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);

    let perm = on_battlefield(&engine, p0, opal_archangel()).expect("seated");
    assert_eq!(pt(&engine, perm), (5, 5));
}
