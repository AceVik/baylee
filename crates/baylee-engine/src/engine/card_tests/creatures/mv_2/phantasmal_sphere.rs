//! `cards/creatures/mv_2/phantasmal_sphere.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn phantasmal_sphere() -> CardIndex {
    card_index("4230c7a3-4591-411e-afab-d2bfb58446de")
}

/// Phantasmal Sphere ({1}{U} 0/1 Creature — Illusion, Partial):
/// - Flying
/// - "At the beginning of your upkeep, put a +1/+1 counter on this creature, then sacrifice this creature unless you pay {1} for each +1/+1 counter on it."
#[test]
fn phantasmal_sphere_gains_counter_and_requires_mana_per_counter_or_sacrifices() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[phantasmal_sphere(), island()])
        .start();
    keep_mulligans(&mut engine);

    let sphere = on_battlefield(&engine, p0, phantasmal_sphere()).unwrap();
    assert!(keywords(&engine, sphere).contains(KeywordSet::FLYING));

    // Turn 1 upkeep: Phantasmal Sphere puts a +1/+1 counter on itself; pay {1}.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    reach_main_phase(&mut engine, p0);

    assert!(on_battlefield(&engine, p0, phantasmal_sphere()).is_some());
    assert_eq!(pt(&engine, sphere), (1, 2));

    // On P0's following upkeep, decline to pay {2} -> Phantasmal Sphere is sacrificed.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, phantasmal_sphere()).is_some());
}
