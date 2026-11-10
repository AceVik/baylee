//! `cards/enchantments/auras/mv_4/animate_artifact.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The Aura targets only an artifact, never the Elves, and animates Sol Ring.
#[test]
fn animate_artifact_attaches_only_to_an_artifact() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                sol_ring(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[animate_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let rock = on_battlefield(&engine, p0, sol_ring()).expect("Sol Ring is seated");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves is seated");
    attaches_only_to(&mut engine, p0, animate_artifact(), rock, elf);
    assert_eq!(pt(&engine, rock), (1, 1));
}
