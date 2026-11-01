//! `cards/creatures/mv_5/skirk_fire_marshal.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn marshal() -> CardIndex {
    card_index("87105cdf-c28e-4fd9-83d0-0fef1704b174")
}

#[test]
fn skirk_fire_marshal_taps_five_goblins_and_burns_everything_except_itself() {
    let mut e = lk::game(
        SEED,
        &[
            marshal(),
            index::RAGING_GOBLIN,
            index::RAGING_GOBLIN,
            index::RAGING_GOBLIN,
            index::RAGING_GOBLIN,
        ],
        &[grizzly_bears()],
        &[],
        &[],
    );
    let source = lk::obj(&e, lk::P0, marshal());
    let goblins = all_on_battlefield(&e, lk::P0, index::RAGING_GOBLIN);
    activate(&mut e, lk::P0, marshal(), 1);
    let Pending::ChooseCards { options, max, .. } = e.pending().clone() else {
        panic!("five Goblins are selected, got {:?}", e.pending());
    };
    assert_eq!(max, 5);
    assert_eq!(options.len(), 5);
    assert!(
        options.contains(&source),
        "the Marshal can pay its own group-tap cost"
    );
    assert!(
        e.apply(
            lk::P0,
            PlayerAction::ChooseObjects {
                objects: goblins.clone()
            }
        )
        .is_err()
    );
    let mut group = goblins.clone();
    group.push(source);
    e.apply(lk::P0, PlayerAction::ChooseObjects { objects: group })
        .unwrap();
    lk::settle(&mut e);

    assert!(lk::tapped(&e, source));
    assert!(all_on_battlefield(&e, lk::P0, index::RAGING_GOBLIN).is_empty());
    assert!(in_graveyard(&e, lk::P1, grizzly_bears()).is_some());
    assert_eq!(
        lk::damage_on(&e, source),
        0,
        "protection prevents its own red damage"
    );
    assert_eq!(lk::life(&e, lk::P0), 10);
    assert_eq!(lk::life(&e, lk::P1), 10);
}

#[test]
fn skirk_fire_marshal_cannot_use_an_already_tapped_goblin() {
    let mut e = lk::game(
        SEED,
        &[
            marshal(),
            index::RAGING_GOBLIN,
            index::RAGING_GOBLIN,
            index::RAGING_GOBLIN,
            index::RAGING_GOBLIN,
        ],
        &[],
        &[],
        &[],
    );
    let goblin = all_on_battlefield(&e, lk::P0, index::RAGING_GOBLIN)[0];
    e.dev_state_mut(lk::P0).unwrap().set_tapped(goblin, true);
    e.refresh_offer();
    assert!(!lk::offered(&e, marshal(), lk::P0, 1));
}
