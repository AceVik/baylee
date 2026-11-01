//! `cards/creatures/mv_7/bruna_the_fading_light.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn bruna() -> CardIndex {
    card_index("bce7e45d-b2c5-43aa-91a6-3a3c24f30850")
}

#[test]
fn bruna_attacks_without_tapping_with_flying_and_vigilance() {
    let p0 = PlayerId::new(0);
    let mut engine = table7(&[bruna()], &[], &[]);
    let angel = on_battlefield(&engine, p0, bruna()).unwrap();
    assert_body(
        &engine,
        angel,
        (5, 7),
        KeywordSet::FLYING.union(KeywordSet::VIGILANCE),
    );
    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.phase == Phase::FirstMain
            && e.state().turn.number > turn
    });
    fight(&mut engine, &[angel], &[]);
    assert!(!is_tapped(&engine, angel));
    assert_eq!(life_of(&engine, 1), 15);
}

#[test]
fn bruna_optionally_returns_only_an_angel_or_human_when_cast() {
    let p0 = PlayerId::new(0);
    for accept in [false, true] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(
                0,
                &[
                    plains(),
                    plains(),
                    plains(),
                    plains(),
                    plains(),
                    plains(),
                    plains(),
                ],
            )
            .hand(0, &[bruna(), serra_angel(), gray_ogre()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let angel = hand_to_graveyard(&mut engine, p0, serra_angel());
        let ogre = hand_to_graveyard(&mut engine, p0, gray_ogre());

        cast_from_hand(&mut engine, p0, bruna());
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseTargets { .. })
        });
        let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
            unreachable!("Bruna's cast trigger targets a graveyard card")
        };
        assert!(options.contains(&angel));
        assert!(!options.contains(&ogre), "an ordinary Ogre is not eligible");
        aim_at(&mut engine, p0, angel);
        pass_until(&mut engine, |e| {
            matches!(
                e.pending(),
                Pending::YesNo {
                    prompt: YesNoPrompt::MayDo,
                    ..
                }
            )
        });
        engine.apply(p0, PlayerAction::YesNo(accept)).unwrap();
        pass_until(&mut engine, stack_is_empty);

        assert!(on_battlefield(&engine, p0, bruna()).is_some());
        assert_eq!(
            engine.state.object(angel).unwrap().zone == Zone::Battlefield,
            accept
        );
        assert_eq!(engine.state.object(ogre).unwrap().zone, Zone::Graveyard);
    }
}
