//! `cards/instants/mv_4/solidarity.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn solidarity() -> CardIndex {
    card_index("7b094a2b-2b69-4a9b-9829-c1aabc53f75e")
}

/// Solidarity (7b094a2b-2b69-4a9b-9829-c1aabc53f75e) — {3}{W} Instant:
/// "Creatures you control get +0/+5 until end of turn."
#[test]
fn solidarity_gives_creatures_you_control_plus_zero_plus_five_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                grizzly_bears(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[solidarity()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let their_bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    cast_from_hand(&mut engine, p0, solidarity());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, my_bears),
        (2, 7),
        "own 2/2 gets +0/+5 until end of turn"
    );
    assert_eq!(
        pt(&engine, my_elf),
        (1, 6),
        "own 1/1 gets +0/+5 until end of turn"
    );
    assert_eq!(
        pt(&engine, their_bears),
        (2, 2),
        "opponent's creature does not get +0/+5"
    );

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, my_bears),
        (2, 2),
        "+0/+5 wears off at end of turn"
    );
    assert_eq!(pt(&engine, my_elf), (1, 1));
}
