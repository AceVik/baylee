//! Leonin Sun Standard pumps only its controller's creatures until cleanup.

use super::*;

fn leonin_sun_standard() -> CardIndex {
    card_index("e235f988-55d2-421b-9cfd-0d30d2727aed")
}

#[test]
fn leonin_sun_standard_pumps_only_ours_and_expires() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[leonin_sun_standard(), plains(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let ours = on_battlefield(&e, p0, llanowar_elves()).unwrap();
    let theirs = on_battlefield(&e, p1, llanowar_elves()).unwrap();
    tap_all_mana(&mut e, p0);
    activate(&mut e, p0, leonin_sun_standard(), 0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, ours), (2, 2));
    assert_eq!(pt(&e, theirs), (1, 1));

    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    assert_eq!(pt(&e, ours), (1, 1));
}
