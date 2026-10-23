#[allow(clippy::wildcard_imports)]
use super::*;

fn opal_titan() -> CardIndex {
    card_index("ce06fe83-a8c5-46b7-b30c-b8189d217741")
}

#[test]
fn opal_titan_becomes_4_4_giant_when_opponent_casts_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[opal_titan()])
        .hand(1, &[grizzly_bears()])
        .battlefield(1, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p1));

    cast_from_hand(&mut engine, p1, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);

    let perm = on_battlefield(&engine, p0, opal_titan()).expect("seated");
    assert_eq!(pt(&engine, perm), (4, 4));
}
