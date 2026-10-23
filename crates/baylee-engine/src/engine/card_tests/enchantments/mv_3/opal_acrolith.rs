#[allow(clippy::wildcard_imports)]
use super::*;

fn opal_acrolith() -> CardIndex {
    card_index("d08b8dd6-3a44-4e86-8ce8-2d0a014a9aa8")
}

#[test]
fn opal_acrolith_becomes_2_4_soldier_on_opponent_creature_cast_and_returns_to_enchantment() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[opal_acrolith()])
        .hand(1, &[grizzly_bears()])
        .battlefield(1, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p1));

    cast_from_hand(&mut engine, p1, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);

    let perm = on_battlefield(&engine, p0, opal_acrolith()).expect("seated");
    assert_eq!(pt(&engine, perm), (2, 4));

    assert!(walk_to_own_main(&mut engine, p0));
    activate(&mut engine, p0, opal_acrolith(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, opal_acrolith()).is_some());
}
