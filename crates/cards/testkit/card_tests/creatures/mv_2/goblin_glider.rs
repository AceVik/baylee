//! `cards/creatures/mv_2/goblin_glider.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn goblin_glider() -> CardIndex {
    card_index("72bce821-46f0-40eb-abdb-575d4305b8fd")
}

/// Goblin Glider ({1}{R} 1/1 Goblin): "Flying; This creature can't block."
#[test]
fn goblin_glider_has_flying_evasion_and_cannot_block_even_flyers() {
    let mut e = lk::game(
        SEED,
        &[mountain(), mountain(), index::WIND_DRAKE],
        &[index::GRIZZLY_BEARS, index::WIND_DRAKE, index::WIND_DRAKE],
        &[goblin_glider()],
        &[],
    );
    cast_from_hand(&mut e, lk::P0, goblin_glider());
    lk::settle(&mut e);
    let glider = lk::obj(&e, lk::P0, goblin_glider());
    let my_drake = lk::obj(&e, lk::P0, index::WIND_DRAKE);
    let their_bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);
    let their_drakes: Vec<ObjectId> = e
        .state()
        .battlefield_seen()
        .filter(|&id| {
            let o = e.state().object(id).unwrap();
            o.controller == lk::P1 && o.card.is_some_and(|c| c.index == index::WIND_DRAKE)
        })
        .collect();

    // Even with Flying, Goblin Glider cannot block an attacking flyer.
    reach_their_main_phase(&mut e, lk::P1);
    let offer = lk::attack(&mut e, &[their_drakes[0]]);
    assert!(!lk::may_block(&offer, glider, their_drakes[0]));
    assert!(lk::may_block(&offer, my_drake, their_drakes[0]));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);

    // Attacking with Goblin Glider evades ground blockers while an untapped flyer may block it.
    reach_their_main_phase(&mut e, lk::P0);
    let offer = lk::attack(&mut e, &[glider]);
    assert!(!lk::may_block(&offer, their_bear, glider));
    assert!(lk::may_block(&offer, their_drakes[1], glider));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 19);
}
