//! `cards/lands/utility/skycoach_waypoint.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn skycoach_waypoint_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(120, forest())
        .battlefield(0, &[skycoach_waypoint()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, skycoach_waypoint()).expect("Waypoint deployed");
    activate(&mut engine, p0, skycoach_waypoint(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

#[test]
fn waypoint_prepares_a_target_with_a_prepare_spell_but_never_an_ordinary_creature() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    for card in [index::EMERITUS_OF_WOE, index::GRIZZLY_BEARS] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::SKYCOACH_WAYPOINT, card])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let target = on_battlefield(&e, p0, card).unwrap();
        let source = on_battlefield(&e, p0, index::SKYCOACH_WAYPOINT).unwrap();
        let state = e.dev_state_mut(p0).unwrap();
        state
            .object_mut(target)
            .unwrap()
            .riders
            .retain(|r| *r != crate::object::Rider::Prepared);
        state.players[0].mana_pool.add(ManaColor::Black, 5);
        e.refresh_offer();
        activate(&mut e, p0, index::SKYCOACH_WAYPOINT, 1);
        aim_at(&mut e, p0, target);
        assert!(
            !e.state()
                .object(target)
                .unwrap()
                .riders
                .contains(&crate::object::Rider::Prepared)
        );
        pass_until(&mut e, stack_is_empty);
        let prepared = card == index::EMERITUS_OF_WOE;
        assert_eq!(
            e.state()
                .object(target)
                .unwrap()
                .riders
                .contains(&crate::object::Rider::Prepared),
            prepared
        );
        assert!(
            !e.state()
                .object(source)
                .unwrap()
                .riders
                .contains(&crate::object::Rider::Prepared)
        );
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("priority")
        };
        assert_eq!(
            legal
                .abilities
                .contains(&(target, crate::choice::PREPARED_CAST)),
            prepared
        );
    }
}
