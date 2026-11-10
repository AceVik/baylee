//! `cards/creatures/mv_4/breezekeeper.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn breezekeeper() -> CardIndex {
    card_index("138439ad-76c6-4d75-8025-e68b94f68fa3")
}

#[test]
fn breezekeeper_has_flying_and_phasing_and_phases_out() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[breezekeeper()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, breezekeeper());
    pass_until(&mut engine, stack_is_empty);

    let bk = on_battlefield(&engine, p0, breezekeeper()).unwrap();
    assert_eq!(pt(&engine, bk), (4, 4));
    assert!(keywords(&engine, bk).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, bk).contains(KeywordSet::PHASING));

    let phased_out = |engine: &Engine<RegistryLookup>| {
        engine
            .state()
            .object(bk)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    };
    assert!(!phased_out(&engine), "phased in when it enters");

    // The opponent's untap step is not its controller's.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        !phased_out(&engine),
        "only its controller's untap phases it"
    );
    // Its controller's next untap step phases it out.
    reach_their_main_phase(&mut engine, p0);
    assert!(phased_out(&engine), "phased out before p0 untaps");
    // It stays out through the opponent's turn and phases in at p0's next untap.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        phased_out(&engine),
        "the opponent's untap does not bring it back"
    );
    reach_their_main_phase(&mut engine, p0);
    assert!(!phased_out(&engine), "phased in again at p0's next untap");
}
