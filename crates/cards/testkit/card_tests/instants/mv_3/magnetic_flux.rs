//! `cards/instants/mv_3/magnetic_flux.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "d50f756d-9ab6-4597-86ae-80beb70879ac"
fn magnetic_flux() -> CardIndex {
    card_index("d50f756d-9ab6-4597-86ae-80beb70879ac")
}

/// Magnetic Flux ({2}{U}): grants flying until end of turn ONLY to artifact
/// creatures you control (not non-artifact creatures and not opponent's
/// artifact creatures), and wears off next turn.
#[test]
fn magnetic_flux_grants_flying_only_to_your_artifact_creatures_until_eot() {
    let mut engine = lk::game(
        SEED,
        &[index::OBSIANUS_GOLEM, index::GRIZZLY_BEARS],
        &[index::OBSIANUS_GOLEM],
        &[magnetic_flux()],
        &[],
    );
    let my_golem = lk::obj(&engine, P0, index::OBSIANUS_GOLEM);
    let my_bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let their_golem = lk::obj(&engine, P1, index::OBSIANUS_GOLEM);

    assert!(!keywords(&engine, my_golem).contains(KeywordSet::FLYING));

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 3)]);
    cast_with_floating(&mut engine, P0, magnetic_flux());
    lk::settle(&mut engine);

    assert!(
        keywords(&engine, my_golem).contains(KeywordSet::FLYING),
        "own artifact creature gained flying"
    );
    assert!(
        !keywords(&engine, my_bears).contains(KeywordSet::FLYING),
        "non-artifact creature did not gain flying"
    );
    assert!(
        !keywords(&engine, their_golem).contains(KeywordSet::FLYING),
        "opponent's artifact creature did not gain flying"
    );

    reach_their_main_phase(&mut engine, P1);
    assert!(
        !keywords(&engine, my_golem).contains(KeywordSet::FLYING),
        "flying wore off at end of turn"
    );
}
