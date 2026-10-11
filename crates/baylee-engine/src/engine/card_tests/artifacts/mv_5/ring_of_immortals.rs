//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn ring_of_immortals_requires_an_instant_or_aura_targeting_your_current_permanent() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for case in 0..6 {
        let (mut e, spell, host) = crate::engine::card_tests::counter_helpers::spell_fixture(
            index::RING_OF_IMMORTALS,
            case,
        );
        if case == 4 {
            let s = e.dev_state_mut(p0).unwrap();
            s.move_object(
                host,
                ZoneLocation::Exile(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
            s.object_mut(host).unwrap().kind = crate::object::ObjectKind::Permanent;
            s.move_object(
                host,
                ZoneLocation::Battlefield,
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
        }
        if case == 5 {
            e.dev_state_mut(p0)
                .unwrap()
                .object_mut(host)
                .unwrap()
                .set_controller(p1);
        }
        let s = e.dev_state_mut(p0).unwrap();
        s.players[0].mana_pool.add(ManaColor::Green, 1);
        s.players[0].mana_pool.add(ManaColor::Colorless, 3);
        e.refresh_offer();
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("{:?}", e.pending())
        };
        let allowed = matches!(case, 0 | 3);
        assert_eq!(
            legal.abilities.iter().any(|(id, idx)| *id
                == on_battlefield(&e, p0, index::RING_OF_IMMORTALS).unwrap()
                && *idx == 0),
            allowed,
            "case {case}"
        );
        if allowed {
            activate(&mut e, p0, index::RING_OF_IMMORTALS, 0);
            let Pending::ChooseTargets { options, .. } = e.pending() else {
                panic!("{:?}", e.pending())
            };
            assert_eq!(options, &vec![spell]);
            aim_at(&mut e, p0, spell);
            pass_until(&mut e, stack_is_empty);
            assert_eq!(e.state().object(spell).unwrap().zone, Zone::Graveyard);
            assert_eq!(pt(&e, host), (2, 2));
            assert_eq!(e.state().object(host).unwrap().damage, 0);
        }
    }
}
