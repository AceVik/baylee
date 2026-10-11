//! `cards/lands/restricted/trenzalore_clocktower.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Trenzalore Clocktower: "{T}: Add {U}. Put a time counter on Trenzalore Clocktower." / "{1}{U}, {T}, Remove twelve time counters..."
/// Activating ability 0 adds blue mana to the pool and places a time counter on Trenzalore Clocktower.
#[test]
fn trenzalore_clocktower_taps_for_blue_and_gains_time_counter() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(321, forest())
        .hand(0, &[trenzalore_clocktower()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, trenzalore_clocktower());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, trenzalore_clocktower(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert_eq!(counters_on(&engine, land, CounterKind::Time), 1);
    assert!(is_tapped(&engine, land));
}

#[test]
fn trenzalore_shuffles_both_hand_and_graveyard_before_drawing_seven() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    for has_doctor in [false, true] {
        let creatures = if has_doctor {
            vec![
                trenzalore_clocktower(),
                index::MASKWOOD_NEXUS,
                index::GRIZZLY_BEARS,
            ]
        } else {
            vec![trenzalore_clocktower()]
        };
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &creatures)
            .hand(0, &[index::GRIZZLY_BEARS, index::SERRA_ANGEL])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let tower = on_battlefield(&e, p0, trenzalore_clocktower()).unwrap();
        let grave = hand_to_graveyard(&mut e, p0, index::GRIZZLY_BEARS);
        let total = library_size(&e, p0) + e.state().zones.list(ZoneLocation::Hand(p0)).len() + 1;
        let state = e.dev_state_mut(p0).unwrap();
        state
            .object_mut(tower)
            .unwrap()
            .counters
            .add(CounterKind::Time, 12);
        state.players[0].mana_pool.add(ManaColor::Blue, 2);
        e.refresh_offer();
        if !has_doctor {
            let Pending::Priority { legal, .. } = e.pending() else {
                unreachable!()
            };
            assert!(!legal.abilities.contains(&(tower, 1)));
            continue;
        }
        activate(&mut e, p0, trenzalore_clocktower(), 1);
        assert_eq!(e.state().object(tower).unwrap().zone, Zone::Exile);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().zones.list(ZoneLocation::Hand(p0)).len(), 7);
        assert_eq!(library_size(&e, p0), total - 7);
        assert!(e.state().zones.list(ZoneLocation::Graveyard(p0)).is_empty());
        assert!(matches!(
            e.state().object(grave).unwrap().zone,
            Zone::Hand | Zone::Library
        ));
    }
}
