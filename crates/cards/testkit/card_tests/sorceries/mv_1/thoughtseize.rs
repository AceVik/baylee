//! `cards/sorceries/mv_1/thoughtseize.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Thoughtseize reveals the entire hand, but the caster chooses a nonland.
#[test]
fn thoughtseize_reveals_the_hand_and_the_caster_selects_the_discard() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut engine = Duel::table(472, forest(), 3)
        .battlefield(0, &[swamp()])
        .hand(0, &[thoughtseize()])
        .hand(1, &[forest(), llanowar_elves(), sol_ring()])
        .hand(2, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));
    let before = engine.state().zones.list(ZoneLocation::Hand(p1)).clone();
    let land = in_hand(&engine, p1, forest()).unwrap();
    let ring = in_hand(&engine, p1, sol_ring()).unwrap();
    let life = engine.state().players[0].life;
    cast_from_hand(&mut engine, p0, thoughtseize());
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending()
    else {
        unreachable!()
    };
    assert_eq!((*player, *min, *max), (p0, 1, 1));
    assert_eq!(options.len(), 2);
    assert!(options.contains(&ring));
    assert!(!options.contains(&land));
    assert!(engine.state().journal.entries().iter().any(|entry| {
        matches!(&entry.event, GameEvent::Revealed { player, cards }
            if *player == p1 && *cards == before)
    }));
    assert_eq!(
        engine.state().players[0].life,
        life,
        "life loss waits for the discard"
    );
    for (seat, objects) in [
        (p1, vec![ring]),
        (p2, vec![ring]),
        (p0, vec![land]),
        (p0, vec![]),
    ] {
        assert!(
            engine
                .apply(seat, PlayerAction::ChooseObjects { objects })
                .is_err()
        );
    }
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .unwrap();
    assert!(stack_is_empty(&engine));
    assert!(in_graveyard(&engine, p1, sol_ring()).is_some());
    assert!(in_hand(&engine, p1, llanowar_elves()).is_some());
    assert!(in_hand(&engine, p2, lightning_bolt()).is_some());
    assert_eq!(engine.state().players[0].life, life - 2);
}

/// Empty or land-only hands still cost two life and never ask an empty choice.
#[test]
fn thoughtseize_without_a_nonland_still_loses_two_life() {
    for cards in [vec![], vec![forest(), forest()]] {
        let (me, them) = (PlayerId::new(0), PlayerId::new(1));
        let mut engine = Duel::new(472, forest())
            .battlefield(0, &[swamp()])
            .hand(0, &[thoughtseize()])
            .hand(1, &cards)
            .start();
        keep_mulligans(&mut engine);
        assert!(walk_to_own_main(&mut engine, me));
        let life = engine.state().players[0].life;
        let before = engine.state().zones.list(ZoneLocation::Hand(them)).clone();
        cast_from_hand(&mut engine, me, thoughtseize());
        engine.apply(me, PlayerAction::ChoosePlayer(them)).unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(engine.state().players[0].life, life - 2);
        assert_eq!(*engine.state().zones.list(ZoneLocation::Hand(them)), before);
    }
}

#[test]
fn thoughtseize_can_target_yourself_and_discard_your_own_nonland() {
    let me = PlayerId::new(0);
    let mut engine = Duel::new(472, forest())
        .battlefield(0, &[swamp()])
        .hand(0, &[thoughtseize(), sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, me));
    let ring = in_hand(&engine, me, sol_ring()).unwrap();
    cast_from_hand(&mut engine, me, thoughtseize());
    engine.apply(me, PlayerAction::ChoosePlayer(me)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            me,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .unwrap();
    assert!(in_graveyard(&engine, me, sol_ring()).is_some());
}
