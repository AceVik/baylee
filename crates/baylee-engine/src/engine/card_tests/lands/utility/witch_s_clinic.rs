//! `cards/lands/utility/witch_s_clinic.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Witch's Clinic: "{T}: Add {C}." / "{2}, {T}: Target commander gains lifelink until end of turn."
/// Under `Coverage::Partial`, targeting a commander is unsupported and the lifelink ability is omitted.
/// Witch's Clinic is played as a land and taps for colorless mana, offering no lifelink activation.
#[test]
fn witch_s_clinic_taps_for_colorless_and_omits_unsupported_lifelink() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(99, forest())
        .hand(0, &[witch_s_clinic()])
        .battlefield(0, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let clinic = play_land(&mut engine, p0, witch_s_clinic());
    assert!(!is_tapped(&engine, clinic));

    // Everything but the land under test: `tap_all_mana` takes printed mana
    // abilities as well as the CR 305.6 shortcut (#159), so tapping it would
    // remove the very offer this asserts on.
    tap_mana_except(&mut engine, p0, clinic);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(clinic, 0)),
        "colorless mana ability is offered"
    );
    assert!(
        !legal.abilities.iter().any(|(s, i)| *s == clinic && *i == 1),
        "unsupported commander lifelink ability is omitted"
    );

    activate(&mut engine, p0, witch_s_clinic(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1
    );
    assert!(is_tapped(&engine, clinic));
}
