//! Nim Lasher counts only artifacts its controller controls.

use super::*;

fn nim_lasher() -> CardIndex {
    card_index("0203e250-d60e-46c6-9c58-d620f0377be0")
}

#[test]
fn nim_lasher_gains_power_when_my_artifact_enters() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[nim_lasher(), forest()])
        .battlefield(1, &[quiet_artifact()])
        .hand(0, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let lasher = on_battlefield(&e, p0, nim_lasher()).unwrap();
    assert_eq!(pt(&e, lasher), (1, 1));
    cast_from_hand(&mut e, p0, quiet_artifact());
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, lasher), (2, 1), "their artifact does not count");
}
