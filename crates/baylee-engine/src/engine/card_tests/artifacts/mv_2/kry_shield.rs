//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn kry_shield_prevents_both_damage_kinds_and_freezes_the_mana_value_bonus() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                index::KRY_SHIELD,
                index::GRIZZLY_BEARS,
                index::FOREST,
                index::FOREST,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    tap_all_mana(&mut e, p0);
    let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    activate(&mut e, p0, index::KRY_SHIELD, 0);
    aim_at(&mut e, p0, bear);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, bear), (2, 4));
    for combat in [false, true] {
        let state = e.dev_state_mut(p0).unwrap();
        let mut work = crate::damage::DamageWork::new(
            state,
            vec![crate::damage::Assignment {
                source: bear,
                source_version: Some(state.object(bear).unwrap().version),
                recipient: crate::event::DamageTarget::Player(p1),
                amount: 3,
                is_combat: combat,
            }],
        );
        assert!(work.advance(state).is_none());
        assert_eq!(work.dealt(), 0);
    }
    reach_their_main_phase(&mut e, p1);
    assert_eq!(pt(&e, bear), (2, 2));
    let state = e.dev_state_mut(p0).unwrap();
    let mut work = crate::damage::DamageWork::new(
        state,
        vec![crate::damage::Assignment {
            source: bear,
            source_version: Some(state.object(bear).unwrap().version),
            recipient: crate::event::DamageTarget::Player(p1),
            amount: 1,
            is_combat: false,
        }],
    );
    assert!(work.advance(state).is_none());
    assert_eq!(work.dealt(), 1);
}
