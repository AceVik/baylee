//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn ayesha_counters_an_artifact_activation_unless_its_controller_pays_white() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for pay in [false, true] {
        let (mut e, ability) =
            crate::engine::card_tests::counter_helpers::artifact_activation(index::AYESHA_TANAKA);
        let before = hand_size(&e, p1);
        e.dev_state_mut(p0).unwrap().players[1]
            .mana_pool
            .add(ManaColor::White, 1);
        e.refresh_offer();
        activate(&mut e, p0, index::AYESHA_TANAKA, 0);
        aim_at(&mut e, p0, ability);
        pass_until(&mut e, |e| {
            matches!(
                e.pending(),
                Pending::YesNo {
                    prompt: YesNoPrompt::PayMana { .. },
                    ..
                }
            )
        });
        assert!(
            matches!(e.pending(),Pending::YesNo{player,prompt:YesNoPrompt::PayMana{cost},..} if *player==p1 && *cost==baylee_core::mana!("{W}"))
        );
        e.apply(p1, PlayerAction::YesNo(pay)).unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(hand_size(&e, p1), before + usize::from(pay));
        assert_eq!(
            e.state().players[1].mana_pool.available(ManaColor::White),
            u32::from(!pay)
        );
    }
}
