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

#[test]
fn recantation_partial_never_uses_a_target_without_a_verse_counter() {
    use crate::engine::card_tests::legends_kit as lk;
    let p0 = PlayerId::new(0);
    let mut e = lk::game_with(
        SEED,
        &[recantation()],
        &[forest(), sol_ring()],
        &[],
        &[],
        |e| lk::yes_no(e, false),
    );
    lk::float(&mut e, p0, &[(ManaColor::Blue, 1)]);
    assert!(!lk::offered(&e, recantation(), p0, 1));
    let id = lk::obj(&e, p0, recantation());
    e.dev_state_mut(p0)
        .unwrap()
        .object_mut(id)
        .unwrap()
        .counters
        .add(baylee_cards_dsl::counters::VERSE, 1);
    e.refresh_offer();
    assert!(lk::offered(&e, recantation(), p0, 1));
    activate(&mut e, p0, recantation(), 1);
    lk::aim(&mut e, &[], &[]);
    lk::settle(&mut e);
    assert!(
        in_graveyard(&e, p0, recantation()).is_some(),
        "up to one includes zero targets"
    );
}
