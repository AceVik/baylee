//! `cards/artifacts/mv_2/time_sieve.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Time Sieve: five artifacts, five questions, and five **different**
/// artifacts.
///
/// "Sacrifice five artifacts" is written as five `CostPart::Sacrifice`
/// parts, one permanent each, and the thing that would make that spelling
/// wrong is a cost that let one artifact answer twice — a Sieve paid for with
/// the same Sol Ring five times is a card that costs one artifact. So the
/// board has exactly the six it needs (the Sieve is an artifact too and may
/// be one of the five, CR 701.16a) and the assertion is on what is left
/// standing afterwards, not on the extra turn alone.
#[test]
fn time_sieve_sacrifices_five_different_artifacts_for_its_extra_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(391, forest())
        .battlefield(
            0,
            &[
                time_sieve(),
                quiet_artifact(),
                quiet_artifact(),
                quiet_artifact(),
                quiet_artifact(),
                quiet_artifact(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let before = engine.state().zones.list(ZoneLocation::Battlefield).len();
    activate(&mut engine, p0, time_sieve(), 0);
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the five sacrifice questions are all answerable"
    );

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Battlefield).len(),
        before - 5,
        "five permanents left the battlefield, so no artifact answered twice"
    );
    assert_eq!(
        engine.state().extra_turns.len(),
        1,
        "and the extra turn is queued"
    );
}
