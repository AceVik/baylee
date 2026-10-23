#[allow(clippy::wildcard_imports)]
use super::*;

fn opal_gargoyle() -> CardIndex {
    card_index("efef2d33-5354-4fce-880e-cb91fa60f730")
}

#[test]
fn opal_gargoyle_becomes_2_2_gargoyle_with_flying_when_opponent_casts_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[opal_gargoyle()])
        .hand(1, &[grizzly_bears()])
        .battlefield(1, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p1));

    cast_from_hand(&mut engine, p1, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);

    let perm = on_battlefield(&engine, p0, opal_gargoyle()).expect("seated");
    assert_eq!(pt(&engine, perm), (2, 2));
}
