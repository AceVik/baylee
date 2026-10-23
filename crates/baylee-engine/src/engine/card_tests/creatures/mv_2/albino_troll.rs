#[allow(clippy::wildcard_imports)]
use super::*;

fn albino_troll() -> CardIndex {
    card_index("02079e18-ac14-4255-95c4-741eb76c4799")
}

#[test]
fn albino_troll_regenerates_from_lethal_damage() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[albino_troll(), lightning_bolt()])
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, albino_troll());
    pass_until(&mut engine, stack_is_empty);

    let troll = on_battlefield(&engine, p0, albino_troll()).expect("seated");
    assert_eq!(pt(&engine, troll), (3, 3));

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: troll,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, p0, lightning_bolt());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![troll],
                    players: vec![],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, albino_troll()).is_some());
    assert!(
        engine
            .state()
            .object(troll)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );
    assert_eq!(engine.state().object(troll).unwrap().damage, 0);
}
