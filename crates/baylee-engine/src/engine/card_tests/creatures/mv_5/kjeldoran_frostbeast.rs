//! `cards/creatures/mv_5/kjeldoran_frostbeast.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn kjeldoran_frostbeast() -> CardIndex {
    card_index("39050823-ccda-43c0-818d-d5db15d82b99")
}

/// Kjeldoran Frostbeast (39050823-ccda-43c0-818d-d5db15d82b99) — {3}{G}{W} 2/4 Creature — Elemental Beast:
/// - "At end of combat, destroy all creatures blocking or blocked by this creature."
#[test]
fn kjeldoran_frostbeast_destroys_creatures_blocking_or_blocked_by_it_at_end_of_combat_and_spares_others()
 {
    let mut engine = lk::game(
        SEED,
        &[kjeldoran_frostbeast()],
        &[
            index::WALL_OF_STONE,
            index::HILL_GIANT,
            index::GRIZZLY_BEARS,
        ],
        &[],
        &[],
    );
    let frostbeast = lk::obj(&engine, P0, kjeldoran_frostbeast());
    let wall = lk::obj(&engine, P1, index::WALL_OF_STONE);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);

    // P0 attacks with 2/4 Kjeldoran Frostbeast; P1 blocks it with 0/8 Wall of Stone.
    // Combat damage (2) does not kill the 0/8 Wall, so the end-of-combat trigger is what destroys it.
    let _ = lk::attack(&mut engine, &[frostbeast]);
    lk::block(&mut engine, &[(wall, frostbeast)]);
    lk::through_combat(&mut engine);

    assert!(
        in_graveyard(&engine, P1, index::WALL_OF_STONE).is_some(),
        "0/8 Wall of Stone blocking Kjeldoran Frostbeast is destroyed at end of combat"
    );
    assert!(
        on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_some(),
        "uninvolved Grizzly Bears is untouched"
    );

    // Advance to P0's next turn so Frostbeast untaps, then to P1's turn where P1 attacks with Hill Giant.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    reach_their_main_phase(&mut engine, P1);

    let _ = lk::attack(&mut engine, &[giant]);
    lk::block(&mut engine, &[(frostbeast, giant)]);
    lk::through_combat(&mut engine);

    assert!(
        in_graveyard(&engine, P1, index::HILL_GIANT).is_some(),
        "3/3 Hill Giant blocked by 2/4 Kjeldoran Frostbeast survives 2 combat damage and is destroyed at end of combat"
    );
    assert!(
        on_battlefield(&engine, P0, kjeldoran_frostbeast()).is_some(),
        "2/4 Kjeldoran Frostbeast survives 3 combat damage"
    );
    assert!(
        on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_some(),
        "uninvolved Grizzly Bears remains on the battlefield"
    );
}
