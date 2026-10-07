//! `cards/lands/utility/arcane_lighthouse.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Arcane Lighthouse: "{T}: Add {C}." / "{1}, {T}: Until end of turn, creatures your opponents control lose hexproof and shroud and can't have hexproof or shroud."
/// Under `Coverage::Partial`, the continuous effect strips existing hexproof and shroud from opponent creatures until end of turn.
/// Activating ability 1 removes hexproof from an opponent's creature upon resolution.
#[test]
fn arcane_lighthouse_removes_hexproof_from_opponent_creatures() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(127, forest())
        .battlefield(0, &[arcane_lighthouse(), forest()])
        .battlefield(1, &[carnage_tyrant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tyrant = on_battlefield(&engine, p1, carnage_tyrant()).expect("Carnage Tyrant deployed");
    assert!(keywords(&engine, tyrant).contains(KeywordSet::HEXPROOF));

    let lighthouse = on_battlefield(&engine, p0, arcane_lighthouse()).unwrap();
    tap_mana_except(&mut engine, p0, lighthouse);
    activate(&mut engine, p0, arcane_lighthouse(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(!keywords(&engine, tyrant).contains(KeywordSet::HEXPROOF));
    assert!(is_tapped(&engine, lighthouse));
}
