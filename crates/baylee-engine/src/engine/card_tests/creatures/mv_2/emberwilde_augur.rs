//! `cards/creatures/mv_2/emberwilde_augur.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;
use baylee_core::generated::index;

fn emberwilde_augur() -> CardIndex {
    card_index("39bf60b2-eeee-48f3-b7b0-6cc2e038afb1")
}

#[test]
fn emberwilde_augur_sacrifices_itself_only_during_its_controllers_upkeep() {
    let mut e = lk::game(
        SEED,
        &[emberwilde_augur()],
        &[index::JACE_THE_MIND_SCULPTOR],
        &[],
        &[],
    );
    assert!(!lk::offered(&e, emberwilde_augur(), lk::P0, 0));
    lk::to_step(&mut e, lk::P1, Step::Upkeep);
    assert!(!lk::offered(&e, emberwilde_augur(), lk::P0, 0));
    pass_until(&mut e, |e| {
        e.state().turn.active == lk::P0
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == lk::P0)
    });
    assert!(lk::offered(&e, emberwilde_augur(), lk::P0, 0));

    activate(&mut e, lk::P0, emberwilde_augur(), 0);
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = e.pending().clone()
    else {
        panic!("a player or planeswalker is targeted: {:?}", e.pending());
    };
    assert!(options.contains(&lk::obj(&e, lk::P1, index::JACE_THE_MIND_SCULPTOR)));
    assert!(player_options.contains(&lk::P1));
    assert!(on_battlefield(&e, lk::P0, emberwilde_augur()).is_some());
    lk::aim(&mut e, &[], &[lk::P1]);
    assert!(in_graveyard(&e, lk::P0, emberwilde_augur()).is_some());
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 17);
}
