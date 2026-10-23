#[allow(clippy::wildcard_imports)]
use super::*;

fn persecute() -> CardIndex {
    card_index("676c4cfa-f929-43aa-95cc-df81a596c2a2")
}

#[test]
fn persecute_reveals_target_player_hand_and_discards_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[persecute()])
        .hand(1, &[wild_elephant()])
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear_id = in_hand(&engine, p1, wild_elephant()).expect("in hand");
    cast_from_hand(&mut engine, p0, persecute());
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
                    objects: vec![bear_id],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, wild_elephant()).is_some());
}
