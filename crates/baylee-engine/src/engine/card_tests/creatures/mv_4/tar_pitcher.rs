//! `cards/creatures/mv_4/tar_pitcher.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn tar_pitcher() -> CardIndex {
    card_index("e6991ab8-36f7-40c4-ad37-06e847d92664")
}

#[test]
fn tar_pitcher_taps_and_sacrifices_a_goblin_to_deal_two() {
    let mut e = lk::game(
        SEED,
        &[tar_pitcher(), index::RAGING_GOBLIN],
        &[grizzly_bears()],
        &[],
        &[],
    );
    let pitcher = lk::obj(&e, lk::P0, tar_pitcher());
    let goblin = lk::obj(&e, lk::P0, index::RAGING_GOBLIN);
    let target = lk::obj(&e, lk::P1, grizzly_bears());
    activate(&mut e, lk::P0, tar_pitcher(), 0);
    lk::aim(&mut e, &[target], &[]);
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        panic!("the cost chooses a Goblin: {:?}", e.pending());
    };
    assert!(options.contains(&goblin) && options.contains(&pitcher));
    e.apply(
        lk::P0,
        PlayerAction::ChooseObjects {
            objects: vec![goblin],
        },
    )
    .unwrap();
    assert!(lk::tapped(&e, pitcher));
    assert!(in_graveyard(&e, lk::P0, index::RAGING_GOBLIN).is_some());
    lk::settle(&mut e);
    assert!(in_graveyard(&e, lk::P1, grizzly_bears()).is_some());
    assert!(on_battlefield(&e, lk::P0, tar_pitcher()).is_some());
}
