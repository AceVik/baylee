//! `cards/instants/mv_1/howl_from_beyond.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Howl from Beyond prints one line — "Target creature gets +X/+0 until end
/// of turn" — and X is the whole of the card, so the scenario names 2 out of
/// a pool four Swamps actually filled: the printed +2/+0 on a 1/1 has to read
/// `(3, 1)`, which neither a fixed pump nor a toughness half could produce.
/// The Elf across the table is the control that keeps the pump on the target
/// that was named, and the Elf beside the Swamps is kept untapped so the pool
/// afterwards is "three black spent and nothing else" rather than a count a
/// mana creature quietly paid into.
#[test]
fn howl_from_beyond_pumps_the_target_it_names_for_the_x_it_was_given() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(811, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), llanowar_elves()])
        .hand(0, &[howl_from_beyond()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");

    // Four Swamps into the pool, and the Elf named as the printing kept back:
    // it is the creature the spell is aimed at, and an Elf tapped for its own
    // mana would make every number below a claim about five.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Swamps, and nothing off the Elf beside them"
    );
    cast_with_floating(&mut engine, p0, howl_from_beyond());

    // CR 601.2b asks for X and CR 601.2c for the target, and the two arrive in
    // whichever order the engine is written to ask them: both are answered out
    // of what the question enumerated rather than in an assumed order.
    let mut aimed = false;
    let mut sized = false;
    for _ in 0..12 {
        if aimed && sized {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, p0, "the casting seat aims it");
                assert_eq!((min, max), (1, 1), "\"target creature\": exactly one");
                assert!(
                    options.contains(&mine) && options.contains(&theirs),
                    "\"target creature\" is any creature, on either side of the \
                     table: {options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![mine],
                        },
                    )
                    .unwrap();
                aimed = true;
            }
            Pending::ChooseNumber {
                player, min, max, ..
            } => {
                assert_eq!(player, p0, "the casting seat names X");
                assert!(min <= 2 && 2 <= max, "X = 2 is not in {min}..={max}");
                engine.apply(player, PlayerAction::ChooseNumber(2)).unwrap();
                sized = true;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Howl is cast: {other:?}"),
        }
    }
    assert!(
        aimed && sized,
        "casting the Howl asks for both X and a target"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, mine),
        (3, 1),
        "+2/+0 for the X that was named — a (1, 3) would be a toughness pump \
         the card does not print"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump lands on the creature that was named and never across the table"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "{{X}}{{B}} with X = 2 is three black out of the four the Swamps made"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and nothing else is in the pool, so the Elf beside them never paid"
    );

    // "Until end of turn": the only reading that tells a duration from a
    // permanent +2/+0 is the same creature on the far side of the turn.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "the pump ended with the turn it was cast in"
    );
}
