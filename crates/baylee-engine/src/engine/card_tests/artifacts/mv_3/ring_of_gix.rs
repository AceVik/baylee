#[allow(clippy::wildcard_imports)]
use super::*;

fn ring_of_gix() -> CardIndex {
    card_index("e01fe9e9-a76c-4909-924c-a0cba403f975")
}

#[test]
fn ring_of_gix_taps_target_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[ring_of_gix()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_from_hand(&mut engine, p0, ring_of_gix());
    pass_until(&mut engine, stack_is_empty);

    let ring = on_battlefield(&engine, p0, ring_of_gix()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: ring,
                ability_index: 1,
            },
        )
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![ogre],
                    players: vec![],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, ogre));
    assert!(is_tapped(&engine, ring));
}
