//! `cards/artifacts/mv_4/portcullis.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
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
