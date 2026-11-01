//! `cards/sorceries/mv_1/encroach.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn encroach() -> CardIndex {
    card_index("b9959d63-5489-4ed1-a04e-9d6ba4a5f68e")
}

#[test]
fn encroach_reveals_hand_and_discards_nonbasic_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[encroach()])
        .battlefield(0, &[swamp()])
        .hand(1, &[badlands(), swamp(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let nonbasic = in_hand(&engine, p1, badlands()).unwrap();
    cast_from_hand(&mut engine, p0, encroach());
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if let Pending::ChooseCards {
        player, options, ..
    } = engine.pending()
    {
        assert_eq!(*player, p0, "the caster chooses from the revealed hand");
        assert_eq!(
            options,
            &vec![nonbasic],
            "basic lands and nonlands are excluded"
        );
    }
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![nonbasic],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, badlands()).is_some());
    assert!(in_hand(&engine, p1, swamp()).is_some());
    assert!(in_hand(&engine, p1, gray_ogre()).is_some());
}
