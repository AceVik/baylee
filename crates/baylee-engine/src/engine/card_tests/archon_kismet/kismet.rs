#[allow(clippy::wildcard_imports)]
use super::*;
#[test]
fn kismet_taps_opponents_artifacts_creatures_and_basic_lands_but_not_ordinary_auras() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::KISMET])
        .hand(0, &[index::SOL_RING, index::GRIZZLY_BEARS, index::FOREST])
        .hand(
            1,
            &[
                index::SOL_RING,
                index::GRIZZLY_BEARS,
                index::FOREST,
                index::HOLY_STRENGTH,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    cast(&mut e, P0, index::SOL_RING);
    pass_until(&mut e, stack_is_empty);
    assert!(!is_tapped(&e, obj(&e, P0, index::SOL_RING)));
    cast(&mut e, P0, index::GRIZZLY_BEARS);
    pass_until(&mut e, stack_is_empty);
    assert!(!is_tapped(&e, obj(&e, P0, index::GRIZZLY_BEARS)));
    let own = play_land(&mut e, P0, index::FOREST);
    assert!(!is_tapped(&e, own));
    next_main(&mut e, P1);
    for c in [index::SOL_RING, index::GRIZZLY_BEARS] {
        cast(&mut e, P1, c);
        pass_until(&mut e, stack_is_empty);
        assert!(is_tapped(&e, obj(&e, P1, c)));
    }
    let land = play_land(&mut e, P1, index::FOREST);
    assert!(is_tapped(&e, land));
    cast(&mut e, P1, index::HOLY_STRENGTH);
    let bear = obj(&e, P1, index::GRIZZLY_BEARS);
    targets(&mut e, vec![bear], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert!(!is_tapped(&e, obj(&e, P1, index::HOLY_STRENGTH)));
}
#[test]
fn kismet_replaces_each_simultaneous_creature_token_entry() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::KISMET])
        .hand(1, &[index::RAISE_THE_ALARM])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    cast(&mut e, P1, index::RAISE_THE_ALARM);
    pass_until(&mut e, stack_is_empty);
    let soldiers: Vec<_> = e
        .state
        .battlefield_seen()
        .filter(|id| {
            e.state.object(*id).is_some_and(|o| {
                o.controller == P1
                    && o.characteristics()
                        .subtypes
                        .contains(subtypes::creature::SOLDIER)
            })
        })
        .collect();
    assert_eq!(soldiers.len(), 2);
    assert!(soldiers.iter().all(|id| is_tapped(&e, *id)));
}
#[test]
fn kismet_uses_entering_controller_even_when_reanimation_steals_an_opponents_card() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::KISMET])
        .hand(0, &[index::GRIZZLY_BEARS])
        .hand(1, &[index::REANIMATE])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let bear = in_hand(&e, P0, index::GRIZZLY_BEARS).unwrap();
    // Initial graveyard placement isolates the destination-controller rule.
    e.state
        .move_object(
            bear,
            ZoneLocation::Graveyard(P0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
    next_main(&mut e, P1);
    cast(&mut e, P1, index::REANIMATE);
    targets(&mut e, vec![bear], vec![]);
    pass_until(&mut e, stack_is_empty);
    let o = e.state.object(bear).unwrap();
    assert_eq!(o.owner, P0);
    assert_eq!(o.controller, P1);
    assert_eq!(o.zone, Zone::Battlefield);
    assert!(is_tapped(&e, bear));
}
#[test]
fn kismet_checks_prospective_types_from_existing_continuous_effects() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::KISMET, index::MYCOSYNTH_LATTICE])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(1, &[index::HOLY_STRENGTH])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    next_main(&mut e, P1);
    cast(&mut e, P1, index::HOLY_STRENGTH);
    let bear = obj(&e, P1, index::GRIZZLY_BEARS);
    targets(&mut e, vec![bear], vec![]);
    pass_until(&mut e, stack_is_empty);
    let aura = obj(&e, P1, index::HOLY_STRENGTH);
    assert!(
        is_tapped(&e, aura),
        "Lattice makes prospective Aura an artifact"
    );
    assert_eq!(e.state.object(aura).unwrap().attached_to, Some(bear));
}
