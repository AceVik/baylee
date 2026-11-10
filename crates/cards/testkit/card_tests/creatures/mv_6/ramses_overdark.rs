//! Printed card behavior through the real engine.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn ramses_targets_enchanted_creatures_and_rechecks_the_attachment() {
    for remove_aura in [false, true] {
        let p0 = PlayerId::new(0);
        let mut e = Duel::new(SEED, forest())
            .battlefield(
                0,
                &[index::RAMSES_OVERDARK, grizzly_bears(), llanowar_elves()],
            )
            .hand(0, &[index::HOLY_STRENGTH])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let bear = on_battlefield(&e, p0, grizzly_bears()).unwrap();
        let elf = on_battlefield(&e, p0, llanowar_elves()).unwrap();
        attach_test_aura(&mut e, p0, bear);
        activate(&mut e, p0, index::RAMSES_OVERDARK, 0);
        let Pending::ChooseTargets { options, .. } = e.pending() else {
            panic!("target")
        };
        assert!(options.contains(&bear));
        assert!(!options.contains(&elf));
        aim_at(&mut e, p0, bear);
        if remove_aura {
            let aura = on_battlefield(&e, p0, index::HOLY_STRENGTH).unwrap();
            bury(&mut e, &[aura]);
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().object(bear).unwrap().zone,
            if remove_aura {
                Zone::Battlefield
            } else {
                Zone::Graveyard
            }
        );
    }
}
