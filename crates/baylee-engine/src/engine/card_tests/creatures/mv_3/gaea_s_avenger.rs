//! Printed card behavior through the real engine.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn gaeas_avenger_counts_only_opponents_artifacts_in_every_zone() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .hand(0, &[index::GAEA_S_AVENGER])
        .battlefield(0, &[index::GAEA_S_AVENGER, sol_ring()])
        .battlefield(1, &[sol_ring(), sol_ring()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let hand = in_hand(&e, p0, index::GAEA_S_AVENGER).unwrap();
    let field = on_battlefield(&e, p0, index::GAEA_S_AVENGER).unwrap();
    for id in [hand, field] {
        let c = crate::layers::recompute(e.state(), e.state().object(id).unwrap()).characteristics;
        assert_eq!((c.power, c.toughness), (Some(3), Some(3)));
    }
    let artifact = on_battlefield(&e, p1, sol_ring()).unwrap();
    bury(&mut e, &[artifact, field]);
    for id in [hand, field] {
        let c = crate::layers::recompute(e.state(), e.state().object(id).unwrap()).characteristics;
        assert_eq!((c.power, c.toughness), (Some(2), Some(2)));
    }
}
