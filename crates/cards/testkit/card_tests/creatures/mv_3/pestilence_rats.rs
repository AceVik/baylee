//! `cards/creatures/mv_3/pestilence_rats.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn pestilence_rats() -> CardIndex {
    card_index("bc6449c8-f0cd-4c07-8acf-436f97e8bbb2")
}

/// Pestilence Rats (bc6449c8-f0cd-4c07-8acf-436f97e8bbb2) — {2}{B} */3 Creature — Rat:
/// "Pestilence Rats's power is equal to the number of other Rats on the battlefield."
#[test]
fn pestilence_rats_power_counts_other_rats_on_both_sides_and_excludes_itself_and_non_rats() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                pestilence_rats(),
                grizzly_bears(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .battlefield(1, &[grizzly_bears(), swamp(), swamp(), swamp()])
        .hand(0, &[pestilence_rats()])
        .hand(1, &[pestilence_rats()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let rats = on_battlefield(&engine, P0, pestilence_rats()).unwrap();

    // 1. Alone with non-Rat creatures (Grizzly Bears): does not count itself or non-Rats -> 0/3.
    assert_eq!(
        pt(&engine, rats),
        (0, 3),
        "Pestilence Rats does not count itself or non-Rat creatures"
    );

    // 2. P0 casts a second Pestilence Rats (another Rat you control) -> 1/3.
    cast_from_hand(&mut engine, P0, pestilence_rats());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, rats), (1, 3));

    // 3. P1 casts a Pestilence Rats (another Rat an opponent controls) -> 2/3.
    reach_their_main_phase(&mut engine, P1);
    cast_from_hand(&mut engine, P1, pestilence_rats());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, rats),
        (2, 3),
        "Pestilence Rats counts other Rats across the entire battlefield"
    );
}
