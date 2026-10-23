#[allow(clippy::wildcard_imports)]
use super::*;

fn rumbling_crescendo() -> CardIndex {
    card_index("2e7e19c2-5399-433d-9653-d956f11838f5")
}

#[test]
fn rumbling_crescendo_adds_verse_counter_and_destroys_target_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[rumbling_crescendo(), mountain()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);

    let crescendo_id = on_battlefield(&engine, p0, rumbling_crescendo()).expect("crescendo on bf");
    assert_eq!(
        engine
            .state()
            .object(crescendo_id)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::VERSE),
        1
    );

    let their_forest = on_battlefield(&engine, p1, forest()).expect("forest on bf");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, rumbling_crescendo(), 1);
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![their_forest],
                    players: vec![],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, forest()).is_none());
    assert!(in_graveyard(&engine, p1, forest()).is_some());
}

#[test]
fn rumbling_crescendo_partial_never_uses_a_target_without_a_verse_counter() {
    use crate::engine::card_tests::legends_kit as lk;
    let p0 = PlayerId::new(0);
    let mut e = lk::game_with(
        SEED,
        &[rumbling_crescendo()],
        &[forest(), sol_ring()],
        &[],
        &[],
        |e| lk::yes_no(e, false),
    );
    lk::float(&mut e, p0, &[(ManaColor::Red, 1)]);
    assert!(!lk::offered(&e, rumbling_crescendo(), p0, 1));
    let id = lk::obj(&e, p0, rumbling_crescendo());
    e.dev_state_mut(p0)
        .unwrap()
        .object_mut(id)
        .unwrap()
        .counters
        .add(baylee_cards_dsl::counters::VERSE, 1);
    e.refresh_offer();
    assert!(lk::offered(&e, rumbling_crescendo(), p0, 1));
    activate(&mut e, p0, rumbling_crescendo(), 1);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("target")
    };
    let excluded = lk::obj(&e, PlayerId::new(1), sol_ring());
    assert!(!options.contains(&excluded));
    lk::aim(&mut e, &[], &[]);
    lk::settle(&mut e);
    assert!(
        in_graveyard(&e, p0, rumbling_crescendo()).is_some(),
        "up to one includes zero targets"
    );
}
