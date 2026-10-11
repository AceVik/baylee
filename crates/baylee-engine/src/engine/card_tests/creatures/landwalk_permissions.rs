//! Printed as-though effects change blocking, not the attacker's abilities.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn ur_drago_and_lord_magnus_ignore_only_the_named_landwalk_while_present() {
    for (legend, ignored, other) in [
        (
            index::UR_DRAGO,
            KeywordSet::SWAMPWALK,
            KeywordSet::FORESTWALK,
        ),
        (
            index::LORD_MAGNUS,
            KeywordSet::PLAINSWALK,
            KeywordSet::SWAMPWALK,
        ),
        (
            index::LORD_MAGNUS,
            KeywordSet::FORESTWALK,
            KeywordSet::ISLANDWALK,
        ),
    ] {
        let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[legend, grizzly_bears()])
            .battlefield(1, &[grizzly_bears(), plains(), island(), swamp(), forest()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let source = on_battlefield(&e, p0, legend).unwrap();
        let attacker = on_battlefield(&e, p0, grizzly_bears()).unwrap();
        let blocker = on_battlefield(&e, p1, grizzly_bears()).unwrap();
        for (extra, allowed) in [
            (KeywordSet::EMPTY, true),
            (other, false),
            (KeywordSet::FLYING, false),
        ] {
            let state = e.dev_state_mut(p0).unwrap();
            state.object_mut(attacker).unwrap().base_mut().keywords = ignored.union(extra);
            state.refresh_characteristics();
            assert_eq!(
                crate::combat::can_block(e.state(), p1, blocker, attacker),
                allowed
            );
            assert!(keywords(&e, attacker).contains(ignored));
        }
        let state = e.dev_state_mut(p0).unwrap();
        state.object_mut(attacker).unwrap().base_mut().keywords = ignored;
        state.refresh_characteristics();
        assert!(crate::combat::can_block(e.state(), p1, blocker, attacker));
        kill(&mut e, source);
        assert!(!crate::combat::can_block(e.state(), p1, blocker, attacker));
        assert!(keywords(&e, attacker).contains(ignored));
    }
}
