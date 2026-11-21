//! `cards/instants/mv_6/into_thin_air.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "06df923f-916e-49f4-960a-7b3fb108c704"
fn into_thin_air() -> CardIndex {
    card_index("06df923f-916e-49f4-960a-7b3fb108c704")
}

/// Into Thin Air ({5}{U}, Affinity for artifacts): costs {1} less ONLY for
/// each artifact YOU control (ignoring opponent's artifacts), and returns
/// target artifact (excluding non-artifacts) to its owner's hand.
#[test]
fn into_thin_air_uses_affinity_for_your_artifacts_and_bounces_target_artifact() {
    let mut engine = lk::game(
        SEED,
        &[sol_ring(), index::ORNITHOPTER, quiet_artifact()],
        &[sol_ring(), quiet_artifact(), index::GRIZZLY_BEARS],
        &[into_thin_air()],
        &[],
    );
    let their_ring = lk::obj(&engine, P1, sol_ring());
    let their_bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    // 3 own artifacts reduce {5}{U} to {2}{U} (opponent's 2 do NOT count).
    lk::float(&mut engine, P0, &[(ManaColor::Blue, 2)]);
    assert!(
        !lk::castable(&engine, P0, into_thin_air()),
        "only {{1}}{{U}} is short of reduced cost {{2}}{{U}}"
    );

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 1)]);
    cast_with_floating(&mut engine, P0, into_thin_air());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&their_ring));
    assert!(
        !options.contains(&their_bears),
        "non-artifact creature is excluded: {options:?}"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![their_ring],
                players: vec![],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert_eq!(
        lk::pool_total(&engine, P0),
        0,
        "spent reduced cost {{2}}{{U}}"
    );
    assert!(in_hand(&engine, P1, sol_ring()).is_some());
}
