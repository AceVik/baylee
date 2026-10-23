#[allow(clippy::wildcard_imports)]
use super::*;

fn ostracize() -> CardIndex {
    card_index("43975c07-103c-4664-b7e6-55bb29d85183")
}

#[test]
fn ostracize_discards_chosen_creature_card_from_opponent_hand() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp()])
        .hand(0, &[ostracize()])
        .hand(1, &[gray_ogre(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre_in_hand = in_hand(&engine, p1, gray_ogre()).unwrap();
    cast_from_hand(&mut engine, p0, ostracize());
    let Pending::ChoosePlayer { options, .. } = engine.pending() else {
        panic!("opponent target");
    };
    assert!(options.contains(&p1));
    assert!(!options.contains(&p0));
    assert!(engine.apply(p0, PlayerAction::ChoosePlayer(p0)).is_err());
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("the caster must choose a revealed creature card");
    };
    assert_eq!(player, p0, "caster chooses the discarded creature");
    assert_eq!((min, max), (1, 1));
    assert_eq!(options, vec![ogre_in_hand]);
    let land = in_hand(&engine, p1, plains()).unwrap();
    assert!(!options.contains(&land));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ogre_in_hand],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, gray_ogre()).is_some());
    assert!(in_hand(&engine, p1, plains()).is_some());
}
