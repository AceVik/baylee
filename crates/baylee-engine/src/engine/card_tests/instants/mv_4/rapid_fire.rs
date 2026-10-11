//! Printed card behavior through the real engine.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn rapid_fire_grants_one_instance_only_when_rampage_is_absent() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for (card, n) in [(grizzly_bears(), 2), (index::AERATHI_BERSERKER, 3)] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[card, plains(), plains(), plains(), plains()])
            .battlefield(1, &[giant_spider(), giant_spider()])
            .hand(0, &[index::RAPID_FIRE])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let a = on_battlefield(&e, p0, card).unwrap();
        let base = pt(&e, a);
        cast_from_hand(&mut e, p0, index::RAPID_FIRE);
        aim_at(&mut e, p0, a);
        pass_until(&mut e, stack_is_empty);
        assert!(keywords(&e, a).contains(KeywordSet::FIRST_STRIKE));
        let blockers: Vec<_> = e
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .copied()
            .filter(|id| e.state().object(*id).unwrap().controller == p1)
            .collect();
        attack_and_collect_blocks(&mut e, a, p1);
        e.apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: blockers.iter().map(|b| (*b, a)).collect(),
            },
        )
        .unwrap();
        assert_eq!(e.state().zones.list(ZoneLocation::Stack).len(), 1);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(pt(&e, a), (base.0 + n, base.1 + n));
    }
}
