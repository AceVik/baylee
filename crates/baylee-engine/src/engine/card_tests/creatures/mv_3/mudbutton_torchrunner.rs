//! `cards/creatures/mv_3/mudbutton_torchrunner.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn mudbutton_torchrunner() -> CardIndex {
    card_index("045b2b84-96bd-46fb-8aae-a33968e741c7")
}

#[test]
fn mudbutton_torchrunner_deals_three_to_any_target_when_it_dies() {
    let mut e = lk::game(SEED, &[mudbutton_torchrunner()], &[], &[], &[]);
    let torchrunner = lk::obj(&e, lk::P0, mudbutton_torchrunner());
    kill(&mut e, torchrunner);
    let Pending::ChooseTargets { player_options, .. } = e.pending().clone() else {
        panic!("the dies trigger targets: {:?}", e.pending());
    };
    assert!(player_options.contains(&lk::P0) && player_options.contains(&lk::P1));
    lk::aim(&mut e, &[], &[lk::P1]);
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 17);
    assert!(in_graveyard(&e, lk::P0, mudbutton_torchrunner()).is_some());
}
