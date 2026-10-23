#[allow(clippy::wildcard_imports)]
use super::*;

fn hidden_predators() -> CardIndex {
    card_index("3b143227-f373-4403-b246-ee1c9a0dc85a")
}

fn flesh_reaver() -> CardIndex {
    card_index("a143fdfe-f1aa-4eab-8c81-8c72a4dc1695")
}

#[test]
fn hidden_predators_becomes_4_4_beast_when_opponent_controls_power_4_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hidden_predators()])
        .hand(1, &[flesh_reaver()])
        .battlefield(1, &[swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p1));

    cast_from_hand(&mut engine, p1, flesh_reaver());
    pass_until(&mut engine, stack_is_empty);

    let perm = on_battlefield(&engine, p0, hidden_predators()).expect("seated");
    assert_eq!(pt(&engine, perm), (4, 4));
}
