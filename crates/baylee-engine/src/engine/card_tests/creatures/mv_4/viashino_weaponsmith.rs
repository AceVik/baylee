#[allow(clippy::wildcard_imports)]
use super::*;

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
