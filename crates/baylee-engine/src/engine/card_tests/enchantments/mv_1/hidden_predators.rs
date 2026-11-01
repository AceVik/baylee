#[allow(clippy::wildcard_imports)]
use super::*;

fn hidden_predators() -> CardIndex {
    card_index("3b143227-f373-4403-b246-ee1c9a0dc85a")
}

#[test]
fn hidden_predators_becomes_4_4_beast_when_opponent_controls_power_4_creature() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hidden_predators(), rootbreaker_wurm()])
        .hand(1, &[index::AIR_ELEMENTAL])
        .battlefield(
            1,
            &[
                wild_elephant(),
                island(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let hidden = on_battlefield(&engine, p0, hidden_predators()).unwrap();
    assert!(
        engine
            .state()
            .object(hidden)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::ENCHANTMENT),
        "your power-six and opponent's power-three creatures do not qualify"
    );
    assert!(walk_to_own_main(&mut engine, p1));

    cast_from_hand(&mut engine, p1, index::AIR_ELEMENTAL);
    pass_until(&mut engine, stack_is_empty);

    let perm = on_battlefield(&engine, p0, hidden_predators()).expect("seated");
    assert_eq!(pt(&engine, perm), (4, 4));
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
            .contains(baylee_core::generated::subtypes::creature::BEAST)
    );
}
