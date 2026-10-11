//! Printed card behavior through the real engine.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn enchanted_source_prevention_obeys_its_damage_scope() {
    for enchanted in [false, true] {
        for combat in [false, true] {
            let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
            let mut e = Duel::new(SEED, forest())
                .battlefield(0, &[index::WALL_OF_PUTRID_FLESH])
                .battlefield(1, &[grizzly_bears()])
                .hand(0, &[index::HOLY_STRENGTH])
                .start();
            keep_mulligans(&mut e);
            reach_main_phase(&mut e, p0);
            let target = on_battlefield(&e, p0, index::WALL_OF_PUTRID_FLESH).unwrap();
            let source = on_battlefield(&e, p1, grizzly_bears()).unwrap();
            if enchanted {
                attach_test_aura(&mut e, p0, source);
            }
            let prevented = enchanted;
            assert_eq!(
                deal_test_damage(&mut e, source, target, combat),
                u32::from(!prevented)
            );
        }
    }
}
