//! `cards/creatures/mv_3/teferi_s_drake.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn teferi_s_drake() -> CardIndex {
    card_index("3605c5ac-3b35-41a5-9266-286fb1d11716")
}

/// Teferi's Drake (`{2}{U}`, `Coverage::Implemented`): 3/2 Drake with flying
/// and phasing.
#[test]
fn teferi_s_drake_has_3_2_flying_and_phases_out_and_in() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(); 3])
        .hand(0, &[teferi_s_drake()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, teferi_s_drake());
    pass_until(&mut engine, stack_is_empty);
    let drake = on_battlefield(&engine, p0, teferi_s_drake()).unwrap();
    assert_eq!(pt(&engine, drake), (3, 2));
    assert!(keywords(&engine, drake).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, drake).contains(KeywordSet::PHASING));

    reach_their_main_phase(&mut engine, p1);
    assert!(
        !engine
            .state()
            .object(drake)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT),
        "the opponent's untap step does not phase it"
    );
    reach_their_main_phase(&mut engine, p0);
    assert!(
        engine
            .state()
            .object(drake)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !engine
            .state()
            .object(drake)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
}
