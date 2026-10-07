//! `cards/creatures/mv_2/capashen_knight.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Capashen Knight` prints `KeywordSet::FIRST_STRIKE` and an activated pump ability
/// `{{1}}{{W}}: This creature gets +1/+0 until end of turn` under `Coverage::Implemented`.
/// Activating the ability on an open board of two Plains verifies that `pt` increases
/// from (1, 1) to (2, 1) while retaining `KeywordSet::FIRST_STRIKE`.
#[test]
fn capashen_knight_pumps_power_with_first_strike() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1604, forest())
        .battlefield(0, &[capashen_knight(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let knight = on_battlefield(&engine, p0, capashen_knight()).expect("knight is on battlefield");
    assert_eq!(pt(&engine, knight), (1, 1));
    assert!(keywords(&engine, knight).contains(KeywordSet::FIRST_STRIKE));

    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, capashen_knight(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, knight), (2, 1));
    assert!(keywords(&engine, knight).contains(KeywordSet::FIRST_STRIKE));
}
