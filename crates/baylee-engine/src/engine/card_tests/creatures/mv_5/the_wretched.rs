//! Printed card behavior through the real engine.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn wretched_takes_only_its_surviving_blockers_and_releases_them_on_departure() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::THE_WRETCHED])
        .battlefield(1, &[giant_spider(), grizzly_bears()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let demon = on_battlefield(&e, p0, index::THE_WRETCHED).unwrap();
    let spider = on_battlefield(&e, p1, giant_spider()).unwrap();
    let bear = on_battlefield(&e, p1, grizzly_bears()).unwrap();
    attack_and_collect_blocks(&mut e, demon, p1);
    e.apply(
        p1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(spider, demon)],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(e.state().object(spider).unwrap().controller, p0);
    assert_eq!(e.state().object(bear).unwrap().controller, p1);
    kill(&mut e, demon);
    assert_eq!(e.state().object(spider).unwrap().controller, p1);
}
