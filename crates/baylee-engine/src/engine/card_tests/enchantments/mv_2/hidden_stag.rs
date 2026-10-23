#[allow(clippy::wildcard_imports)]
use super::*;

fn hidden_stag() -> CardIndex {
    card_index("70c32155-dc07-45cb-bceb-e19d3ebc915d")
}

#[test]
fn hidden_stag_becomes_creature_on_opponent_land_and_enchantment_on_your_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hidden_stag()])
        .hand(0, &[forest()])
        .hand(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p1));

    play_land(&mut engine, p1, forest());
    pass_until(&mut engine, stack_is_empty);

    let perm = on_battlefield(&engine, p0, hidden_stag()).expect("seated");
    assert_eq!(pt(&engine, perm), (3, 2));

    assert!(walk_to_own_main(&mut engine, p0));
    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, hidden_stag()).is_some());
}
