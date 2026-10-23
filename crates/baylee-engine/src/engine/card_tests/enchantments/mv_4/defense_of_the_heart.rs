#[allow(clippy::wildcard_imports)]
use super::*;

fn defense_of_the_heart() -> CardIndex {
    card_index("e7e1b166-9267-426d-897d-24903327b48d")
}

#[test]
fn defense_of_the_heart_sacrifices_itself_and_tutors_two_creatures_to_battlefield() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, gray_ogre())
        .hand(0, &[defense_of_the_heart()])
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .battlefield(1, &[grizzly_bears(), gray_ogre(), earth_elemental()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, defense_of_the_heart());
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, max, .. } = engine.pending().clone() else {
        panic!("expected ChooseCards");
    };
    let take = usize::try_from(max).unwrap_or(1).min(2);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: options[..take].to_vec(),
            },
        )
        .unwrap();
    if let Pending::ChooseCards { options, .. } = engine.pending().clone() {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .unwrap();
    }
    reach_their_main_phase(&mut engine, p0);

    assert!(on_battlefield(&engine, p0, defense_of_the_heart()).is_none());
    assert!(on_battlefield(&engine, p0, gray_ogre()).is_some());
}
