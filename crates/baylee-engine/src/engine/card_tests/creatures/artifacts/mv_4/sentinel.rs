//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn sentinel_sets_only_its_own_toughness_and_keeps_a_negative_targets_power() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for negative in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::SENTINEL])
            .battlefield(1, &[grizzly_bears()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let sentinel = on_battlefield(&e, p0, index::SENTINEL).unwrap();
        let blocker = on_battlefield(&e, p1, grizzly_bears()).unwrap();
        e.dev_state_mut(p0)
            .unwrap()
            .object_mut(sentinel)
            .unwrap()
            .counters
            .add(CounterKind::P1P1, 1);
        attack_and_collect_blocks(&mut e, sentinel, p1);
        e.apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, sentinel)],
            },
        )
        .unwrap();
        activate(&mut e, p0, index::SENTINEL, 0);
        aim_at(&mut e, p0, blocker);
        if negative {
            let state = e.dev_state_mut(p0).unwrap();
            let timestamp = state.next_timestamp();
            let filter = crate::effects::EffectFilter::object(state, blocker);
            state.effects.register(crate::effects::ContinuousEffect {
                id: baylee_core::ids::EffectId::new(0),
                source: None,
                controller: p0,
                origin: crate::effects::EffectOrigin::Resolution,
                layer: baylee_cards_dsl::Layer::PtModify,
                timestamp,
                duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
                filter,
                modifier: baylee_cards_dsl::Modifier::ModifyPT(-4, 0),
            });
            state.refresh_characteristics();
            assert_eq!(pt(&e, blocker), (-2, 2));
        }
        pass_until(&mut e, stack_is_empty);
        if negative {
            assert_eq!(
                e.state().object(sentinel).unwrap().zone,
                Zone::Graveyard,
                "-2 + 1 base + 1 counter = 0"
            );
        } else {
            assert_eq!(pt(&e, sentinel), (2, 4));
            assert_eq!(pt(&e, blocker), (2, 2));
            reach_their_main_phase(&mut e, p1);
            assert_eq!(pt(&e, sentinel), (2, 4), "setting persists");
        }
    }
}
