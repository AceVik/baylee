#[allow(clippy::wildcard_imports)]
use super::*;

fn duress() -> CardIndex {
    card_index("33d405ea-7a9a-4970-b70f-9c05d90dd6f0")
}

#[test]
fn duress_discards_noncreature_nonland_card_from_opponent_hand() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, llanowar_elves())
        .hand(0, &[duress()])
        .hand(
            1,
            &[
                counterspell(),
                baylee_core::generated::index::GRIZZLY_BEARS,
                forest(),
            ],
        )
        .battlefield(0, &[swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cs_id = in_hand(&engine, p1, counterspell()).expect("in hand");
    cast_from_hand(&mut engine, p0, duress());
    if matches!(engine.pending(), Pending::ChoosePlayer { .. }) {
        engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if let Pending::ChooseCards { options, .. } = engine.pending().clone() {
        assert_eq!(options, vec![cs_id]);
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![cs_id],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, counterspell()).is_some());
    assert!(in_hand(&engine, p1, baylee_core::generated::index::GRIZZLY_BEARS).is_some());
    assert!(in_hand(&engine, p1, forest()).is_some());
}
