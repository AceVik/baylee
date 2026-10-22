//! `cards/creatures/mv_3/teferi_s_imp.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn teferi_s_imp() -> CardIndex {
    card_index("e4826967-b2f9-4f32-8411-d243edf41082")
}

/// Teferi's Imp (`{2}{U}`, `Coverage::Partial`): 1/1 Imp with flying and
/// phasing.
#[test]
fn teferi_s_imp_has_1_1_flying_and_phases_out_and_in() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(); 3])
        .hand(0, &[teferi_s_imp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, teferi_s_imp());
    pass_until(&mut engine, stack_is_empty);
    let imp = on_battlefield(&engine, p0, teferi_s_imp()).unwrap();
    assert_eq!(pt(&engine, imp), (1, 1));
    assert!(keywords(&engine, imp).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, imp).contains(KeywordSet::PHASING));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        engine
            .state()
            .object(imp)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !engine
            .state()
            .object(imp)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
}
