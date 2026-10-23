#[allow(clippy::wildcard_imports)]
use super::*;

fn thopter_squadron() -> CardIndex {
    card_index("f1482ca4-610b-48fe-ac23-4526637bb72a")
}

#[test]
fn thopter_squadron_spawns_and_sacrifices_thopter_tokens() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[thopter_squadron()])
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
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let squadron = cast_resolved(&mut engine, p0, thopter_squadron());
    assert_eq!(pt(&engine, squadron), (3, 3));

    // Activate ability 0 ({1}, Remove a +1/+1 counter: Create a 1/1 Thopter token).
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: squadron,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, squadron), (2, 2));

    let token = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| engine.state().object(id).unwrap().card.is_none())
        .expect("Thopter token created");
    assert_eq!(pt(&engine, token), (1, 1));

    // Activate ability 1 ({1}, Sacrifice another Thopter: Put a +1/+1 counter on this creature).
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: squadron,
                ability_index: 1,
            },
        )
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![token],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, squadron), (3, 3));
}
