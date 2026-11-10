#[allow(clippy::wildcard_imports)]
use super::*;

fn chimeric_staff() -> CardIndex {
    card_index("771f5e29-b4d1-4466-8510-ce3f89ad7173")
}

#[test]
fn chimeric_staff_becomes_x_x_construct_artifact_creature_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[chimeric_staff()])
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, chimeric_staff());
    pass_until(&mut engine, stack_is_empty);
    let staff = on_battlefield(&engine, p0, chimeric_staff()).expect("seated");

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: staff,
                ability_index: 0,
            },
        )
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseNumber { .. }) {
        engine.apply(p0, PlayerAction::ChooseNumber(4)).unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, staff), (4, 4));
    assert!(
        engine
            .state()
            .object(staff)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::CONSTRUCT)
    );
    assert!(
        engine
            .state()
            .object(staff)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    let chars = engine.state().object(staff).unwrap().characteristics();
    assert!(chars.types.contains(TypeSet::ARTIFACT));
    assert!(!chars.types.contains(TypeSet::CREATURE));
}
