//! `cards/creatures/artifacts/mv_4/energizer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Energizer` prints `{{2}}, {{T}}: Put a +1/+1 counter on this creature.` on a 2/2
/// artifact creature juggernaut with `Coverage::Implemented`.
/// In this scenario, two `forest()` lands provide the mana to pay the activation cost.
/// Because its ability is not a mana ability, `tap_all_mana` leaves `Energizer` untapped so that
/// it can pay its own `TapSelf` cost. Resolving the ability places a `+1/+1` counter on it,
/// increasing its power and toughness to 3/3.
#[test]
fn energizer_taps_and_pays_two_mana_to_gain_a_counter() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), energizer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let jugg = on_battlefield(&engine, p0, energizer()).expect("energizer is seated");
    assert_eq!(pt(&engine, jugg), (2, 2));
    assert_eq!(counters_on(&engine, jugg, CounterKind::P1P1), 0);
    assert!(!is_tapped(&engine, jugg));

    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    assert!(
        !is_tapped(&engine, jugg),
        "`Energizer` has no mana ability, so `tap_all_mana` does not tap it"
    );

    activate(&mut engine, p0, energizer(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, jugg));
    assert_eq!(counters_on(&engine, jugg, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, jugg), (3, 3));
}
