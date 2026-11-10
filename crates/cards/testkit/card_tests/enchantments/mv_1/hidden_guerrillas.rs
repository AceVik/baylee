#[allow(clippy::wildcard_imports)]
use super::*;

fn hidden_guerrillas() -> CardIndex {
    card_index("4337f35b-baab-4640-9e1c-05f61c49bbdd")
}

#[test]
fn hidden_guerrillas_becomes_5_3_soldier_when_opponent_casts_artifact() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hidden_guerrillas(), forest()])
        .hand(0, &[sol_ring()])
        .hand(1, &[sol_ring(), grizzly_bears()])
        .battlefield(1, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, sol_ring());
    pass_until(&mut engine, stack_is_empty);
    let perm = on_battlefield(&engine, p0, hidden_guerrillas()).unwrap();
    assert!(
        engine
            .state()
            .object(perm)
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
            .object(perm)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::ENCHANTMENT)
    );

    cast_from_hand(&mut engine, p1, sol_ring());
    pass_until(&mut engine, stack_is_empty);

    let perm = on_battlefield(&engine, p0, hidden_guerrillas()).expect("seated");
    assert_eq!(pt(&engine, perm), (5, 3));
    assert!(keywords(&engine, perm).contains(KeywordSet::TRAMPLE));
    let chars = engine.state().object(perm).unwrap().characteristics();
    assert!(chars.types.contains(TypeSet::CREATURE));
    assert!(!chars.types.contains(TypeSet::ENCHANTMENT));
    assert!(
        chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::SOLDIER)
    );
}
