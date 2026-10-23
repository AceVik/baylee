#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn wizard_mentor() -> CardIndex {
    card_index("a1fb1b77-a49f-4447-bbe0-156407f192e2")
}

#[test]
fn wizard_mentor_returns_self_and_target_creature_you_control_to_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[wizard_mentor(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("ogre on bf");
    activate(&mut engine, p0, wizard_mentor(), 0);
    aim_at(&mut engine, p0, ogre);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, wizard_mentor()).is_none());
    assert!(on_battlefield(&engine, p0, gray_ogre()).is_none());
    assert!(in_hand(&engine, p0, wizard_mentor()).is_some());
    assert!(in_hand(&engine, p0, gray_ogre()).is_some());
}

#[test]
fn wizard_mentor_does_not_offer_an_opponents_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = lk::game(
        SEED,
        &[wizard_mentor(), gray_ogre()],
        &[grizzly_bears()],
        &[],
        &[],
    );
    let bear = lk::obj(&engine, p1, grizzly_bears());
    activate(&mut engine, p0, wizard_mentor(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(!options.contains(&bear));
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![bear],
                    players: vec![]
                }
            )
            .is_err()
    );
    let ogre = lk::obj(&engine, p0, gray_ogre());
    lk::aim(&mut engine, &[ogre], &[]);
    lk::settle(&mut engine);
    assert!(in_hand(&engine, p0, wizard_mentor()).is_some());
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_some());
}
