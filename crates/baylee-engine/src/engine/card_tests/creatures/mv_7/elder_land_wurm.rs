//! Printed card behavior through the real engine.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn blocking_removes_defender_across_turns_but_not_zone_changes() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[grizzly_bears()])
        .battlefield(1, &[index::ELDER_LAND_WURM])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let bear = on_battlefield(&e, p0, grizzly_bears()).unwrap();
    let wurm = on_battlefield(&e, p1, index::ELDER_LAND_WURM).unwrap();
    assert!(keywords(&e, wurm).contains(KeywordSet::DEFENDER));
    let blocks = attack_and_collect_blocks(&mut e, bear, p1);
    assert!(blocks.iter().any(|b| b.blocker == wurm));
    e.apply(
        p1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(wurm, bear)],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(!keywords(&e, wurm).contains(KeywordSet::DEFENDER));
    assert!(keywords(&e, wurm).contains(KeywordSet::TRAMPLE));
    reach_their_main_phase(&mut e, p1);
    assert!(!keywords(&e, wurm).contains(KeywordSet::DEFENDER));
    let state = e.dev_state_mut(p1).unwrap();
    state
        .move_object(
            wurm,
            ZoneLocation::Hand(p1),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    assert!(
        state
            .object(wurm)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::DEFENDER)
    );
}
