//! `cards/instants/mv_5/feedback_bolt.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

// oracle_id = "2fc5ebd9-e4af-4e2b-a2cc-a13fe051c5cb"
fn feedback_bolt() -> CardIndex {
    card_index("2fc5ebd9-e4af-4e2b-a2cc-a13fe051c5cb")
}

/// Feedback Bolt ({4}{R}): targets ONLY a player or planeswalker (excluding
/// creatures) and deals damage equal to the number of artifacts YOU control
/// (ignoring opponent's artifacts).
#[test]
fn feedback_bolt_damages_player_or_planeswalker_by_your_artifact_count() {
    let mut engine = table(
        SEED,
        &[feedback_bolt(), feedback_bolt()],
        &[mountain(), mountain(), mountain(), mountain(), mountain()],
        &[sol_ring(), index::ORNITHOPTER, quiet_artifact()],
        &[
            sol_ring(),
            index::GRIZZLY_BEARS,
            index::JACE_THE_MIND_SCULPTOR,
        ],
    );
    let bears = on_battlefield(&engine, them(), index::GRIZZLY_BEARS).unwrap();
    let jace = on_battlefield(&engine, them(), index::JACE_THE_MIND_SCULPTOR).unwrap();

    // 1. Target opponent -> 3 damage for P0's 3 artifacts (not 4).
    let (obj_opts, player_opts) = cast_at_player(&mut engine, feedback_bolt(), them());
    assert!(obj_opts.contains(&jace), "planeswalker is a legal target");
    assert!(
        !obj_opts.contains(&bears),
        "creature is not a legal target: {obj_opts:?}"
    );
    assert!(player_opts.contains(&them()));
    assert_eq!(
        life_of(&engine, them()),
        17,
        "3 damage for 3 controlled artifacts"
    );

    // 2. Next turn target Jace (3 loyalty) -> 3 damage puts Jace in graveyard.
    reach_their_main_phase(&mut engine, them());
    reach_their_main_phase(&mut engine, me());
    cast_at(&mut engine, feedback_bolt(), jace);
    assert!(in_graveyard(&engine, them(), index::JACE_THE_MIND_SCULPTOR).is_some());
}
