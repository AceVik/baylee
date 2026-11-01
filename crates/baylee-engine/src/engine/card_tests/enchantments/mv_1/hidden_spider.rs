#[allow(clippy::wildcard_imports)]
use super::*;

fn hidden_spider() -> CardIndex {
    card_index("b0709570-a145-4c3c-ba56-f06d25753855")
}

fn fog_bank() -> CardIndex {
    card_index("3a2bb47d-228d-499e-a34b-9e10d99e9b2c")
}

#[test]
fn hidden_spider_becomes_3_5_spider_with_reach_when_opponent_casts_flyer() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hidden_spider(), island(), island()])
        .hand(0, &[fog_bank()])
        .hand(1, &[fog_bank(), grizzly_bears()])
        .battlefield(1, &[island(), island(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, fog_bank());
    pass_until(&mut engine, stack_is_empty);
    let hidden = on_battlefield(&engine, p0, hidden_spider()).unwrap();
    assert!(
        engine
            .state()
            .object(hidden)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::ENCHANTMENT)
    );
    assert!(walk_to_own_main(&mut engine, p1));
    cast_from_hand(&mut engine, p1, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine
            .state()
            .object(hidden)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::ENCHANTMENT)
    );

    cast_from_hand(&mut engine, p1, fog_bank());
    pass_until(&mut engine, stack_is_empty);

    let perm = on_battlefield(&engine, p0, hidden_spider()).expect("seated");
    assert_eq!(pt(&engine, perm), (3, 5));
    assert!(keywords(&engine, perm).contains(KeywordSet::REACH));
    assert!(
        !engine
            .state()
            .object(perm)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::ENCHANTMENT)
    );
    assert!(
        engine
            .state()
            .object(perm)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::SPIDER)
    );
}
