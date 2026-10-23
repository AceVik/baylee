#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::engine::card_tests::partials_w1b_kit as kit;

fn viashino_weaponsmith() -> CardIndex {
    card_index("f4d17f41-efd6-4173-ab6e-28b6fbcee10d")
}

#[test]
fn viashino_weaponsmith_gets_plus_two_plus_two_when_blocked_by_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[viashino_weaponsmith()])
        .battlefield(1, &[wild_elephant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let smith = on_battlefield(&engine, p0, viashino_weaponsmith()).expect("smith on bf");
    let elephant = on_battlefield(&engine, p1, wild_elephant()).expect("elephant on bf");
    let _ = attack_and_collect_blocks(&mut engine, smith, p1);
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elephant, smith)],
            },
        )
        .unwrap();
    reach_their_main_phase(&mut engine, p1);

    assert!(on_battlefield(&engine, p0, viashino_weaponsmith()).is_some());
    assert!(on_battlefield(&engine, p1, wild_elephant()).is_none());
}

#[test]
fn viashino_weaponsmith_triggers_once_per_blocker_and_pump_expires() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = lk::game(
        SEED,
        &[viashino_weaponsmith()],
        &[grizzly_bears(), gray_ogre()],
        &[],
        &[],
    );
    let smith = lk::obj(&engine, p0, viashino_weaponsmith());
    let bear = lk::obj(&engine, p1, grizzly_bears());
    let ogre = lk::obj(&engine, p1, gray_ogre());
    lk::attack(&mut engine, &[smith]);
    lk::block(&mut engine, &[(bear, smith), (ogre, smith)]);
    lk::settle(&mut engine);
    assert_eq!(pt(&engine, smith), (6, 6));
    kit::drive(
        &mut engine,
        |e| {
            e.state().turn.active == p1
                && e.state().turn.phase == Phase::FirstMain
                && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
        },
        |_, pending| match pending {
            Pending::ChooseNumber {
                max,
                reason: crate::choice::NumberPrompt::CombatDamage { .. },
                ..
            } => Some(PlayerAction::ChooseNumber(*max)),
            _ => None,
        },
    );
    assert_eq!(pt(&engine, smith), (2, 2));
}
