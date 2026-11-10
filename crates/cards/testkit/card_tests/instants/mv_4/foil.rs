//! `cards/instants/mv_4/foil.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn foil() -> CardIndex {
    card_index("7dde4eb6-e9d7-4259-abc2-3af738e0f00f")
}

/// Foil ({2}{U}{U} Instant, Partial):
/// "Counter target spell."
#[test]
fn foil_counters_target_creature_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[foil()])
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    cast_from_hand(&mut engine, p1, index::GRIZZLY_BEARS);
    let spell = on_stack(&engine, index::GRIZZLY_BEARS).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    cast_from_hand(&mut engine, p0, foil());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![spell],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, index::GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, p1, index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&engine, p0, foil()).is_some());
}

/// Foil can also counter a noncreature spell on the stack.
#[test]
fn foil_counters_target_noncreature_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[foil()])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    cast_from_hand(&mut engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    let spell = on_stack(&engine, lightning_bolt()).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    cast_from_hand(&mut engine, p0, foil());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![spell],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 20);
    assert!(in_graveyard(&engine, p1, lightning_bolt()).is_some());
}
