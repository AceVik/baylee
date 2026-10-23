#[allow(clippy::wildcard_imports)]
use super::*;

fn sustainer_of_the_realm() -> CardIndex {
    card_index("2c99d52d-f909-4494-aeee-937ae779fd4a")
}

#[test]
fn sustainer_of_the_realm_gets_toughness_when_blocking() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[sustainer_of_the_realm()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let sustainer = on_battlefield(&engine, p0, sustainer_of_the_realm()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();

    let _ = attack_and_collect_blocks(&mut engine, ogre, p0);
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(sustainer, ogre)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, sustainer), (2, 5));
}
