//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn indestructible_aura_prevents_combat_and_noncombat_damage_until_cleanup() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[grizzly_bears(), plains()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[index::INDESTRUCTIBLE_AURA])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let protected = on_battlefield(&e, p0, grizzly_bears()).unwrap();
    let source = on_battlefield(&e, p1, grizzly_bears()).unwrap();
    cast_from_hand(&mut e, p0, index::INDESTRUCTIBLE_AURA);
    aim_at(&mut e, p0, protected);
    pass_until(&mut e, stack_is_empty);
    for (combat, expected) in [(false, 0), (true, 0), (false, 1)] {
        if expected == 1 {
            reach_their_main_phase(&mut e, p1);
        }
        let state = e.dev_state_mut(p0).unwrap();
        let source_version = Some(state.object(source).unwrap().version);
        let mut work = crate::damage::DamageWork::new(
            state,
            vec![crate::damage::Assignment {
                source,
                source_version,
                recipient: crate::event::DamageTarget::Object(protected),
                amount: 1,
                is_combat: combat,
            }],
        );
        assert!(work.advance(state).is_none());
        assert_eq!(work.dealt(), expected);
    }
}
