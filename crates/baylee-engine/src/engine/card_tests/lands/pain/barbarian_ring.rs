//! `cards/lands/pain/barbarian_ring.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Barbarian Ring prints `{{T}}: Add {{R}}. This land deals 1 damage to you.`
/// and `Threshold — {{R}}, {{T}}, Sacrifice this land: It deals 2 damage to any target.
/// Activate only if there are seven or more cards in your graveyard.`
///
/// Under `Coverage::Partial`, the threshold activation is omitted because the
/// engine condition system cannot count cards in the controller's graveyard.
/// This test seeds seven cards into the graveyard, verifies that only ability
/// 0 is offered, and confirms that activating it adds one red mana to the pool
/// while dealing 1 damage to its controller.
#[test]
fn barbarian_ring_taps_for_red_and_deals_one_damage() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[barbarian_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 7);
    let ring = on_battlefield(&engine, p0, barbarian_ring()).expect("ring on battlefield");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        legal.abilities.iter().filter(|(id, _)| *id == ring).count(),
        1,
        "only the mana ability is offered despite threshold graveyard size"
    );

    let life_before = engine.state().players[0].life;
    activate(&mut engine, p0, barbarian_ring(), 0);

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "{{T}}: Add {{R}}"
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before - 1,
        "deals 1 damage to you"
    );
    assert!(is_tapped(&engine, ring));
}

/// Barbarian Ring: the ability that was not on the card at all.
#[test]
fn barbarian_ring_deals_two_only_once_the_graveyard_is_full() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(11, forest())
        .battlefield(0, &[barbarian_ring(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ring = on_battlefield(&engine, p0, barbarian_ring()).expect("the Ring is in play");

    // Floated first, for the reason Cabal Pit's test spells out: an empty
    // pool withholds the ability by itself.
    tap_mana_except(&mut engine, p0, ring);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("still priority")
    };
    assert!(
        !legal.abilities.contains(&(ring, 1)),
        "an empty graveyard is not threshold"
    );

    seed_graveyard(&mut engine, p0, 7);
    let life = engine.state().players[1].life;
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: ring,
                ability_index: 1,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!("\"any target\", got {:?}", engine.pending())
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: Vec::new(),
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1].life,
        life - 2,
        "\"it deals 2 damage to any target\""
    );
    assert!(
        on_battlefield(&engine, p0, barbarian_ring()).is_none(),
        "sacrificing the land is part of the cost, so it is gone by resolution \
         — and the damage still happens (CR 608.2g)"
    );
}
