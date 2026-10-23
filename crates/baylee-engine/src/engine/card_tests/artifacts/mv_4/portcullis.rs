//! `cards/artifacts/mv_4/portcullis.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn portcullis() -> CardIndex {
    card_index("6230c179-0263-4fa4-b0ff-f2cb621de147")
}

/// Portcullis (`6230c179-0263-4fa4-b0ff-f2cb621de147`) — {4} Artifact (Partial):
/// "Whenever a creature enters, if there are two or more other creatures on the battlefield, exile that creature."
#[test]
fn portcullis_exiles_entering_creature_when_two_other_creatures_exist() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[portcullis(), index::GRIZZLY_BEARS, plains()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[index::SAVANNAH_LIONS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, index::SAVANNAH_LIONS);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, index::SAVANNAH_LIONS).is_none(),
        "entering third creature should be exiled by Portcullis"
    );
    assert_eq!(engine.state().zones.list(ZoneLocation::Exile(P0)).len(), 1);
}

#[test]
fn second_creature_stays_and_third_returns_when_artifact_leaves() {
    let mut e = lk::game(
        SEED,
        &[portcullis(), index::GRIZZLY_BEARS],
        &[],
        &[index::SAVANNAH_LIONS, index::HILL_GIANT, index::DISENCHANT],
        &[],
    );
    lk::float(
        &mut e,
        lk::P0,
        &[(ManaColor::White, 20), (ManaColor::Red, 5)],
    );
    cast_with_floating(&mut e, lk::P0, index::SAVANNAH_LIONS);
    lk::settle(&mut e);
    assert!(
        on_battlefield(&e, lk::P0, index::SAVANNAH_LIONS).is_some(),
        "only one other creature"
    );
    cast_with_floating(&mut e, lk::P0, index::HILL_GIANT);
    lk::settle(&mut e);
    assert!(on_battlefield(&e, lk::P0, index::HILL_GIANT).is_none());
    let artifact = lk::obj(&e, lk::P0, portcullis());
    lk::cast_at(&mut e, lk::P0, index::DISENCHANT, &[artifact], &[]);
    assert!(
        on_battlefield(&e, lk::P0, index::HILL_GIANT).is_some(),
        "linked card returns to owner"
    );
    assert!(e.state().zones.list(ZoneLocation::Exile(lk::P0)).is_empty());
}
