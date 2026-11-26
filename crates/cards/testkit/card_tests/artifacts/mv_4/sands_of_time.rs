//! `cards/artifacts/mv_4/sands_of_time.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn sands_of_time() -> CardIndex {
    card_index("c8f69b48-a3b2-4324-852d-11fad19d6432")
}

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn sol_ring() -> CardIndex {
    card_index("6ad8011d-3471-4369-9d68-b264cc027487")
}

fn glorious_anthem() -> CardIndex {
    card_index("e3886fe8-9b76-4613-8891-4ec74657c087")
}

#[test]
fn sands_of_time_skips_untap_step_and_toggles_tap_state_at_upkeep() {
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), sands_of_time()])
        .battlefield(
            1,
            &[forest(), grizzly_bears(), sol_ring(), glorious_anthem()],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    let p0_forest = on_battlefield(&e, P0, forest()).unwrap();
    let p1_forest = on_battlefield(&e, P1, forest()).unwrap();
    let p1_bears = on_battlefield(&e, P1, grizzly_bears()).unwrap();
    let p1_ring = on_battlefield(&e, P1, sol_ring()).unwrap();
    let p1_anthem = on_battlefield(&e, P1, glorious_anthem()).unwrap();

    e.dev_state_mut(P0).unwrap().set_tapped(p0_forest, true);
    e.dev_state_mut(P1).unwrap().set_tapped(p1_forest, true);
    e.dev_state_mut(P1).unwrap().set_tapped(p1_ring, true);

    reach_their_main_phase(&mut e, P1);
    assert!(!is_tapped(&e, p1_forest), "tapped land untapped on upkeep");
    assert!(
        is_tapped(&e, p1_bears),
        "untapped creature tapped on upkeep"
    );
    assert!(
        !is_tapped(&e, p1_ring),
        "tapped artifact untapped on upkeep"
    );
    assert!(!is_tapped(&e, p1_anthem), "non-ACL enchantment unaffected");
    assert!(
        is_tapped(&e, p0_forest),
        "non-active player permanent unaffected"
    );
}
