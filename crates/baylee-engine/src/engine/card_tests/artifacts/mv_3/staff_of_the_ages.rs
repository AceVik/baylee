//! `cards/artifacts/mv_3/staff_of_the_ages.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn staff_of_the_ages() -> CardIndex {
    card_index("86af21ba-58ad-45e8-a316-5411c870e415")
}

fn shanodin_dryads() -> CardIndex {
    card_index("998484cc-fefc-4da5-9987-5d6e89599c34")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Staff of the Ages (86af21ba-58ad-45e8-a316-5411c870e415) — {3} Artifact:
/// "Creatures with landwalk abilities can be blocked as though they didn't have those abilities."
#[test]
fn staff_of_the_ages_allows_blocking_creatures_with_basic_landwalk() {
    // 1. Without Staff of the Ages: Shanodin Dryads (forestwalk) cannot be blocked when P1 controls a Forest.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[shanodin_dryads()])
        .battlefield(1, &[forest(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let dryads = on_battlefield(&engine, P0, shanodin_dryads()).unwrap();
    let blocker = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    let blocks = attack_and_collect_blocks(&mut engine, dryads, P1);
    assert!(
        !blocks
            .iter()
            .any(|b| b.blocker == blocker && b.attackers.contains(&dryads)),
        "without Staff of the Ages, forestwalk makes Shanodin Dryads unblockable"
    );

    // 2. With Staff of the Ages on the battlefield: Shanodin Dryads CAN be blocked even though P1 controls a Forest.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[shanodin_dryads()])
        .battlefield(1, &[staff_of_the_ages(), forest(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let dryads = on_battlefield(&engine, P0, shanodin_dryads()).unwrap();
    let blocker = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    let blocks = attack_and_collect_blocks(&mut engine, dryads, P1);
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == blocker && b.attackers.contains(&dryads)),
        "with Staff of the Ages, Shanodin Dryads can be blocked despite forestwalk"
    );
}
