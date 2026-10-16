//! `cards/creatures/artifacts/mv_3/guardians_of_meletis.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn guardians_of_meletis() -> CardIndex {
    card_index("7604aac7-82ae-4f5b-888d-a008754f716a")
}

/// Guardians of Meletis — 0/6 defender: it is never on the attack offer, the
/// plain creature beside it is.
#[test]
fn guardians_of_meletis_defender_cannot_attack() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[guardians_of_meletis(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let guard = on_battlefield(&engine, p0, guardians_of_meletis()).expect("seated");
    let plain = on_battlefield(&engine, p0, quiet_creature()).expect("seated");
    assert_eq!(pt(&engine, guard), (0, 6));
    assert!(keywords(&engine, guard).contains(KeywordSet::DEFENDER));

    let offered = attackers_offered(&mut engine);
    assert!(offered.contains(&plain), "the plain creature may attack");
    assert!(!offered.contains(&guard), "defender cannot");
}
