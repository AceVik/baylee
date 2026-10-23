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
    if matches!(engine.pending(), Pending::ChoosePlayer { .. }) {
        engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![ogre_in_hand],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, gray_ogre()).is_some());
}
