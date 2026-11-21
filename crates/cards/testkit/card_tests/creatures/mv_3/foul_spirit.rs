//! `cards/creatures/mv_3/foul_spirit.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn foul_spirit() -> CardIndex {
    card_index("d8177d08-a603-42e9-9350-890eac05e184")
}

/// Foul Spirit ({2}{B} 3/2 Spirit):
/// "Flying; When this creature enters, sacrifice a land."
#[test]
fn foul_spirit_sacrifices_one_of_your_lands_on_entry_and_flies_over_ground_blockers() {
    let mut e = lk::game(
        SEED,
        &[index::SWAMP, index::SWAMP, forest()],
        &[plains(), index::GRIZZLY_BEARS],
        &[foul_spirit()],
        &[],
    );
    let my_forest = lk::obj(&e, lk::P0, forest());
    let their_plains = lk::obj(&e, lk::P1, plains());
    let their_bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);

    cast_from_hand(&mut e, lk::P0, foul_spirit());
    pass_until(&mut e, |eng| {
        matches!(eng.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = e.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!((player, min, max), (lk::P0, 1, 1));
    assert!(options.contains(&my_forest));
    assert!(!options.contains(&their_plains));
    e.apply(
        lk::P0,
        PlayerAction::ChooseObjects {
            objects: vec![my_forest],
        },
    )
    .unwrap();
    lk::settle(&mut e);

    assert!(in_graveyard(&e, lk::P0, forest()).is_some());
    let spirit = lk::obj(&e, lk::P0, foul_spirit());

    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    let offer = lk::attack(&mut e, &[spirit]);
    assert!(!lk::may_block(&offer, their_bear, spirit));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 17);
}
