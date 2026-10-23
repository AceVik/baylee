#[allow(clippy::wildcard_imports)]
use super::*;

fn umbilicus() -> CardIndex {
    card_index("c9132e72-d016-4cb7-8662-a16b7d932bef")
}

#[test]
fn umbilicus_bounces_chosen_permanent_on_upkeep() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[umbilicus(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("ogre on bf");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ogre],
            },
        )
        .unwrap();
    reach_main_phase(&mut engine, p0);

    assert!(on_battlefield(&engine, p0, gray_ogre()).is_none());
    assert!(in_hand(&engine, p0, gray_ogre()).is_some());
}
