//! `cards/creatures/mv_3/loran_of_the_third_path.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Loran of the Third Path: "{T}: You and **target opponent** each draw a
/// card." An opponent, so the controller is not on offer — and *one* of
/// them, which `PlayerRel::Opponent` could not say.
#[test]
fn loran_draws_for_the_one_opponent_she_named() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(31, plains())
        .battlefield(0, &[loran_of_the_third_path()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let loran = on_battlefield(&engine, p0, loran_of_the_third_path()).expect("Loran deployed");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == loran)
        .expect("Loran's tap ability is offered");
    let (hand0, hand1) = (
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();

    let Pending::ChooseTargets {
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert_eq!(
        player_options,
        vec![p1],
        "\"target opponent\" leaves the controller out (CR 115.1)",
    );
    assert_eq!((min, max), (1, 1));

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand0 + 1,
        "you draw",
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        hand1 + 1,
        "and so does the opponent you named",
    );
}

/// Loran of the Third Path: "When Loran enters, destroy up to one target
/// artifact or enchantment."
#[test]
fn loran_destroys_an_artifact_when_it_enters() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(4607, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .battlefield(1, &[sol_ring()])
        .hand(0, &[loran_of_the_third_path()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ring = on_battlefield(&engine, p1, sol_ring()).expect("their Sol Ring");
    cast_from_hand(&mut engine, p0, loran_of_the_third_path());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .expect("Sol Ring is on offer");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p1, sol_ring()).is_none(),
        "destroyed"
    );
    assert!(in_graveyard(&engine, p1, sol_ring()).is_some());
}
