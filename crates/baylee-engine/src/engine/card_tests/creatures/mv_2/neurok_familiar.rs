//! Neurok Familiar reveals and sorts its top card by artifact type.

use super::*;

fn neurok_familiar() -> CardIndex {
    card_index("8e11d1f7-fa98-4cd2-9912-081e9c9e37ee")
}

fn sorted(library: CardIndex) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, library)
        .battlefield(0, &[island(), island()])
        .hand(0, &[neurok_familiar()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    cast_from_hand(&mut e, p0, neurok_familiar());
    pass_until(&mut e, stack_is_empty);
    e
}

#[test]
fn neurok_familiar_puts_artifact_in_hand_and_nonartifact_in_graveyard() {
    let p0 = PlayerId::new(0);
    let e = sorted(quiet_artifact());
    assert!(in_hand(&e, p0, quiet_artifact()).is_some());
    assert!(in_graveyard(&e, p0, quiet_artifact()).is_none());
    let e = sorted(forest());
    assert!(in_graveyard(&e, p0, forest()).is_some());
    assert_eq!(library_size(&e, p0), 59);
}
