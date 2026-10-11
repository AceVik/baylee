//! `cards/artifacts/vehicles/mv_4/conqueror_s_galleon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn conqueror_s_galleon_casts_as_uncrewed_vehicle_and_cannot_attack() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(306, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), plains(), plains()],
        )
        .hand(0, &[conqueror_s_galleon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, conqueror_s_galleon());
    pass_until(&mut engine, stack_is_empty);

    let galleon = on_battlefield(&engine, p0, conqueror_s_galleon())
        .expect("Conqueror's Galleon on battlefield");
    assert_eq!(pt(&engine, galleon), (2, 10), "printed 2/10 body");

    let t = types(&engine, galleon);
    assert!(t.contains(TypeSet::ARTIFACT), "Galleon is an artifact");
    assert!(
        !t.contains(TypeSet::CREATURE),
        "uncrewed vehicle is not a creature"
    );
    assert!(!t.contains(TypeSet::LAND), "Galleon is not a land");

    // Float mana from remaining Plains and verify that no crew ability is offered.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == galleon),
        "no creatures are available to pay crew 4"
    );

    // Advance to combat and verify that Galleon cannot attack.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on ChooseAttackers");
    };
    assert!(
        !attackers.contains(&galleon),
        "uncrewed vehicle cannot be declared as an attacker"
    );
}

#[test]
fn galleon_crews_and_returns_transformed_only_at_end_of_combat() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                conqueror_s_galleon(),
                baylee_core::generated::index::GRIZZLY_BEARS,
                baylee_core::generated::index::GRIZZLY_BEARS,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let ship = on_battlefield(&e, p0, conqueror_s_galleon()).unwrap();
    let before = identity(&e, ship);
    activate(&mut e, p0, conqueror_s_galleon(), 0);
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        panic!("crew")
    };
    assert_eq!(options.len(), 2);
    e.apply(p0, PlayerAction::ChooseObjects { objects: options })
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(types(&e, ship).contains(TypeSet::CREATURE));
    attack_and_collect_blocks(&mut e, ship, p1);
    assert_eq!(e.state().object(ship).unwrap().face_index, 0);
    e.apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut e, |e| {
        e.state().turn.phase == crate::turn::Phase::SecondMain
            && matches!(e.pending(),Pending::Priority{player,..} if *player==p0)
    });
    assert_eq!(e.state().object(ship).unwrap().face_index, 1);
    assert_ne!(identity(&e, ship), before);
    assert!(types(&e, ship).contains(TypeSet::LAND));
    assert!(!types(&e, ship).contains(TypeSet::CREATURE));
    assert!(!is_tapped(&e, ship));
    activate(&mut e, p0, conqueror_s_galleon(), 0);
    assert_eq!(
        e.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1
    );
}
