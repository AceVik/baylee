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

/// "Whenever enchanted creature deals damage, you gain that much life": the
/// Aura's controller gains when the enchanted creature deals the damage
/// (even to an opponent, in combat or not), and gains nothing when another
/// creature deals it, even to them.
#[test]
fn spirit_link_gains_only_for_the_enchanted_creatures_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for (from_enchanted, combat) in [(true, false), (true, true), (false, false), (false, true)] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[plains(), grizzly_bears()])
            .battlefield(1, &[grizzly_bears()])
            .hand(0, &[index::SPIRIT_LINK])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let mine = on_battlefield(&e, p0, grizzly_bears()).unwrap();
        let theirs = on_battlefield(&e, p1, grizzly_bears()).unwrap();
        cast_from_hand(&mut e, p0, index::SPIRIT_LINK);
        aim_at(&mut e, p0, mine);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().players[0].life, 20);
        assert_eq!(e.state().players[1].life, 20);

        let (source, victim) = if from_enchanted {
            (mine, p1)
        } else {
            (theirs, p0)
        };
        crate::engine::card_tests::history_helpers::damage_player(
            &mut e, source, victim, 3, combat,
        );
        e.apply(p0, PlayerAction::PassPriority).unwrap();
        pass_until(&mut e, stack_is_empty);

        let (me, them) = (e.state().players[0].life, e.state().players[1].life);
        if from_enchanted {
            assert_eq!(
                (me, them),
                (23, 17),
                "combat={combat}: the enchanted creature hit the opponent and its Aura's controller gained 3"
            );
        } else {
            assert_eq!(
                (me, them),
                (17, 20),
                "combat={combat}: a creature the Aura is not on dealt the damage, so no life is gained"
            );
        }
    }
}
