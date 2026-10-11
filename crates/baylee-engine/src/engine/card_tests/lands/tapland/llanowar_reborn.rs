//! `cards/lands/tapland/llanowar_reborn.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Llanowar Reborn prints `This land enters tapped`, `{T}: Add {G}`, and `Graft 1`.
/// When played, Llanowar Reborn enters tapped with one `CounterKind::P1P1` counter, untaps on the subsequent
/// turn cycle, and taps for one green mana at ability index 0.
#[test]
fn llanowar_reborn_enters_tapped_with_counter_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[llanowar_reborn()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, llanowar_reborn());
    assert!(entered_tapped(&engine, land));
    assert_eq!(counters_on(&engine, land, CounterKind::P1P1), 1);

    let from = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > from
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, llanowar_reborn(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, land));
}

#[test]
fn graft_moves_one_counter_only_between_the_two_original_permanents() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    for case in 0..5 {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[llanowar_reborn(), index::DOUBLING_SEASON])
            .hand(0, &[index::GRIZZLY_BEARS])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let land = on_battlefield(&e, p0, llanowar_reborn()).unwrap();
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Green, 2);
        e.refresh_offer();
        cast_with_floating(&mut e, p0, index::GRIZZLY_BEARS);
        pass_until(&mut e, |e| {
            on_battlefield(e, p0, index::GRIZZLY_BEARS).is_some()
        });
        let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
        if case >= 2 {
            let object = if case == 3 { land } else { bear };
            let s = e.dev_state_mut(p0).unwrap();
            s.move_object(
                object,
                ZoneLocation::Exile(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
            if case >= 3 {
                s.object_mut(object).unwrap().kind = crate::object::ObjectKind::Permanent;
                s.move_object(
                    object,
                    ZoneLocation::Battlefield,
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::Effect,
                )
                .unwrap();
            }
        }
        if case == 4 {
            // The returned creature makes a new graft trigger. Decline it,
            // then accept the old trigger for the creature that left.
            pass_until(&mut e, |e| {
                matches!(
                    e.pending(),
                    Pending::YesNo {
                        prompt: crate::choice::YesNoPrompt::MayDo,
                        ..
                    }
                )
            });
            e.apply(p0, PlayerAction::YesNo(false)).unwrap();
        }
        pass_until(&mut e, |e| {
            matches!(
                e.pending(),
                Pending::YesNo {
                    prompt: crate::choice::YesNoPrompt::MayDo,
                    ..
                }
            )
        });
        e.apply(p0, PlayerAction::YesNo(case != 1)).unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            counters_on(&e, land, CounterKind::P1P1),
            if case == 0 { 1 } else { 2 },
            "case {case}"
        );
        assert_eq!(
            counters_on(&e, bear, CounterKind::P1P1),
            if case == 0 { 2 } else { 0 },
            "case {case}"
        );
    }
}
