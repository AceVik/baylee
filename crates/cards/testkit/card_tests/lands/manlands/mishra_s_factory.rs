//! `cards/lands/manlands/mishra_s_factory.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mishra's Factory prints `{T}: Add {C}`, `{1}: This land becomes a 2/2 Assembly-Worker artifact
/// creature until end of turn. It's still a land`, and `{T}: Target Assembly-Worker creature gets +1/+1 until end of turn.`
/// The card is marked `Coverage::Implemented`.
/// Activating ability index 1 animates the land into an untapped 2/2 Assembly-Worker artifact creature,
/// which can subsequently activate ability index 2 targeting itself to become a 3/3 creature.
#[test]
fn mishras_factory_animates_into_assembly_worker_and_pumps_itself() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[mishra_s_factory()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let factory = play_land(&mut engine, p0, mishra_s_factory());
    assert!(!types(&engine, factory).contains(TypeSet::CREATURE));

    tap_all_mana_but(&mut engine, p0, Some(mishra_s_factory()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );

    activate(&mut engine, p0, mishra_s_factory(), 1);
    pass_until(&mut engine, stack_is_empty);

    let t = types(&engine, factory);
    assert!(t.contains(TypeSet::ARTIFACT));
    assert!(t.contains(TypeSet::CREATURE));
    assert!(t.contains(TypeSet::LAND));
    assert_eq!(pt(&engine, factory), (2, 2));
    assert!(!is_tapped(&engine, factory));

    // CR 302.6: the land was played this turn, and the moment it becomes a
    // creature its `{T}` ability is one a creature has — so the pump is not
    // on offer at all, untapped board or not. A land that animates itself is
    // the one shape where summoning sickness reaches something that was
    // never summoned.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(id, ai)| *id == factory && *ai == 2),
        "the pump is offered on the turn the factory arrived"
    );

    // Its next turn: still the same permanent, no longer newly arrived. The
    // animation wore off at end of turn, so it is bought again.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!types(&engine, factory).contains(TypeSet::CREATURE));

    tap_all_mana_but(&mut engine, p0, Some(mishra_s_factory()));
    activate(&mut engine, p0, mishra_s_factory(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, factory), (2, 2));

    activate(&mut engine, p0, mishra_s_factory(), 2);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice");
    };
    assert!(options.contains(&factory));

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![factory],
                players: vec![],
            },
        )
        .unwrap();

    assert!(is_tapped(&engine, factory));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, factory), (3, 3));
}
