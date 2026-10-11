//! `cards/lands/artifacts/power_depot.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The printed mana or land-face ability is exercised through player actions.
#[test]
fn power_depot_enters_tapped_with_counter_and_adds_restricted_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest()).hand(0, &[power_depot()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, power_depot());
    assert!(entered_tapped(&engine, land));
    assert_eq!(counters_on(&engine, land, CounterKind::P1P1), 1);
    let t = types(&engine, land);
    assert!(t.contains(TypeSet::ARTIFACT));
    assert!(t.contains(TypeSet::LAND));

    let from = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > from
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, power_depot(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice");
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 0);
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Blue);
    assert!(is_tapped(&engine, land));
}

#[test]
fn modular_uses_departure_counters_even_if_the_source_returns_and_may_target_an_opponents_creature()
{
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for card in [index::POWER_DEPOT, index::ARCBOUND_RAVAGER] {
        for (decline, recipient) in [(false, p0), (false, p1), (true, p0)] {
            let mut e = Duel::new(SEED, forest())
                .battlefield(0, &[card, index::ORNITHOPTER, index::DOUBLING_SEASON])
                .battlefield(1, &[index::ORNITHOPTER])
                .start();
            keep_mulligans(&mut e);
            reach_main_phase(&mut e, p0);
            let source = on_battlefield(&e, p0, card).unwrap();
            let target = on_battlefield(&e, recipient, index::ORNITHOPTER).unwrap();
            assert_eq!(counters_on(&e, source, CounterKind::P1P1), 2);
            {
                let s = e.dev_state_mut(p0).unwrap();
                crate::replacement::put_counters(s, source, CounterKind::P1P1, 2);
            }
            assert_eq!(counters_on(&e, source, CounterKind::P1P1), 6);
            bury(&mut e, &[source]);
            e.apply(p0, PlayerAction::PassPriority).unwrap();
            let Pending::ChooseTargets { player, .. } = e.pending().clone() else {
                panic!("modular target: {:?}", e.pending())
            };
            aim_at(&mut e, player, target);
            {
                let s = e.dev_state_mut(p0).unwrap();
                s.object_mut(source).unwrap().kind = crate::object::ObjectKind::Permanent;
                s.move_object(
                    source,
                    ZoneLocation::Battlefield,
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::Effect,
                )
                .unwrap();
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
            e.apply(p0, PlayerAction::YesNo(!decline)).unwrap();
            pass_until(&mut e, stack_is_empty);
            assert_eq!(
                counters_on(&e, target, CounterKind::P1P1),
                if decline {
                    0
                } else if recipient == p0 {
                    12
                } else {
                    6
                }
            );
            assert_eq!(
                counters_on(&e, source, CounterKind::P1P1),
                2,
                "new permanent's counters are separate"
            );
        }
    }
}
