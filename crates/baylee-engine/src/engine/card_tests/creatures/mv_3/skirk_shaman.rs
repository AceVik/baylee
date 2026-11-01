//! `cards/creatures/mv_3/skirk_shaman.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn skirk_shaman() -> CardIndex {
    card_index("78d5ce34-f160-4925-a4ce-25a3546d97c7")
}

#[test]
fn skirk_shaman_accepts_red_or_artifact_blockers_and_refuses_a_plain_bear() {
    let mut e = lk::game(
        SEED,
        &[skirk_shaman()],
        &[grizzly_bears(), index::RAGING_GOBLIN, index::SAND_GOLEM],
        &[],
        &[],
    );
    let attacker = lk::obj(&e, lk::P0, skirk_shaman());
    let bear = lk::obj(&e, lk::P1, grizzly_bears());
    let red = lk::obj(&e, lk::P1, index::RAGING_GOBLIN);
    let artifact = lk::obj(&e, lk::P1, index::SAND_GOLEM);
    let offered = lk::attack(&mut e, &[attacker]);
    assert!(!lk::may_block(&offered, bear, attacker));
    assert!(lk::may_block(&offered, red, attacker));
    assert!(lk::may_block(&offered, artifact, attacker));
    assert!(
        e.apply(
            lk::P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bear, attacker)],
            },
        )
        .is_err()
    );
    lk::block(&mut e, &[(red, attacker)]);
    assert_eq!(e.state().combat.blockers_of(attacker), vec![red]);
}
