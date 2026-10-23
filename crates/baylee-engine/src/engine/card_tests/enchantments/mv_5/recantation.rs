#[allow(clippy::wildcard_imports)]
use super::*;

fn recantation() -> CardIndex {
    card_index("02269353-2949-4599-b5a3-f20fdb501285")
}

#[test]
fn recantation_adds_verse_counter_and_bounces_target_permanent() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[recantation(), island()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);

    let recant_id = on_battlefield(&engine, p0, recantation()).expect("recantation on bf");
    assert_eq!(
        engine
            .state()
            .object(recant_id)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::VERSE),
        1
    );

    let bear_id = on_battlefield(&engine, p1, gray_ogre()).expect("ogre on bf");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, recantation(), 1);
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![bear_id],
                    players: vec![],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, gray_ogre()).is_none());
    assert!(in_hand(&engine, p1, gray_ogre()).is_some());
}
