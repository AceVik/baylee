//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn spirit_link_gains_the_aura_controller_life_for_combat_and_other_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for combat in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[plains()])
            .battlefield(1, &[grizzly_bears()])
            .hand(0, &[index::SPIRIT_LINK])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let bear = on_battlefield(&e, p1, grizzly_bears()).unwrap();
        cast_from_hand(&mut e, p0, index::SPIRIT_LINK);
        aim_at(&mut e, p0, bear);
        pass_until(&mut e, stack_is_empty);
        let life = e.state().players[0].life;
        let other = e.state().players[1].life;
        let state = e.dev_state_mut(p0).unwrap();
        let source_version = Some(state.object(bear).unwrap().version);
        let mut work = crate::damage::DamageWork::new(
            state,
            vec![crate::damage::Assignment {
                source: bear,
                source_version,
                recipient: crate::event::DamageTarget::Player(p0),
                amount: 3,
                is_combat: combat,
            }],
        );
        assert!(work.advance(state).is_none());
        assert_eq!(work.dealt(), 3);
        e.apply(p0, PlayerAction::PassPriority).unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().players[0].life, life);
        assert_eq!(e.state().players[1].life, other);
    }
}
