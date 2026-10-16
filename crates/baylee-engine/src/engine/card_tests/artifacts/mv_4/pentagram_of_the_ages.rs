//! `cards/artifacts/mv_4/pentagram_of_the_ages.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn pentagram_of_the_ages() -> CardIndex {
    card_index("265204a9-0ce2-4a0a-a9ea-8537cd94b079")
}

#[test]
fn probe() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[pentagram_of_the_ages(), rocket_launcher(), forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, pentagram_of_the_ages(), 0);
    pass_until(&mut engine, |e| !matches!(e.pending(), Pending::Priority { .. }) || stack_is_empty(e));
    panic!("PROBE {:?}", engine.pending());
}
