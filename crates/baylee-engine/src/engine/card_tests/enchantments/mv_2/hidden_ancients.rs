#[allow(clippy::wildcard_imports)]
use super::*;

fn hidden_ancients() -> CardIndex {
    card_index("dc430e1b-d57f-4035-b518-9bc2e26a6c04")
}

#[test]
fn hidden_ancients_becomes_5_5_treefolk_when_opponent_casts_enchantment() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hidden_ancients(), forest(), forest()])
        .hand(0, &[sylvan_library()])
        .hand(1, &[sylvan_library(), sol_ring()])
        .battlefield(1, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, sylvan_library());
    pass_until(&mut engine, stack_is_empty);
    let hidden = on_battlefield(&engine, p0, hidden_ancients()).unwrap();
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
    cast_from_hand(&mut engine, p1, sol_ring());
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

    cast_from_hand(&mut engine, p1, sylvan_library());
    pass_until(&mut engine, stack_is_empty);

    let perm = on_battlefield(&engine, p0, hidden_ancients()).expect("seated");
    assert_eq!(pt(&engine, perm), (5, 5));
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
            .contains(baylee_core::generated::subtypes::creature::TREEFOLK)
    );
}
