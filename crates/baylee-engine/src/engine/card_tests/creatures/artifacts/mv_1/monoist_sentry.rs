//! `cards/creatures/artifacts/mv_1/monoist_sentry.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn monoist_sentry() -> CardIndex {
    card_index("0b4cb111-ecf7-4ff7-a9e3-c944054f3370")
}

/// "Defender" on a 4/1: it is not offered as an attacker.
#[test]
fn monoist_sentry_cannot_attack() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[monoist_sentry()], &[]);
    let me = on_battlefield(&engine, p0, monoist_sentry()).expect("seated");
    assert_eq!(pt(&engine, me), (4, 1));
    assert!(keywords(&engine, me).contains(KeywordSet::DEFENDER));
    assert!(!attackers_offered(&mut engine).contains(&me));
}
