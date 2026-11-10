#[allow(clippy::wildcard_imports)]
use super::*;

fn congregate() -> CardIndex {
    card_index("d139294c-36bb-4a1e-800d-50c4a4ae9877")
}

#[test]
fn congregate_gains_two_life_per_creature_on_battlefield() {
    for target in [PlayerId::new(0), PlayerId::new(1)] {
        let p0 = PlayerId::new(0);
        let mut engine = Duel::new(SEED, plains())
            .hand(0, &[congregate()])
            .battlefield(
                0,
                &[
                    plains(),
                    plains(),
                    plains(),
                    plains(),
                    grizzly_bears(),
                    gray_ogre(),
                ],
            )
            .battlefield(1, &[grizzly_bears()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        cast_from_hand(&mut engine, p0, congregate());
        engine
            .apply(p0, PlayerAction::ChoosePlayer(target))
            .unwrap();
        pass_until(&mut engine, stack_is_empty);

        assert_eq!(engine.state().players[usize::from(target.get())].life, 26);
        assert_eq!(
            engine.state().players[usize::from(1 - target.get())].life,
            20
        );
    }
}
