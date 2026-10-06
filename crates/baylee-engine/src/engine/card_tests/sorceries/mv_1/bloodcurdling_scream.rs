//! `cards/sorceries/mv_1/bloodcurdling_scream.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Bloodcurdling Scream prints one line — "Target creature gets +X/+0 until
/// end of turn" for {X}{B} — and the whole card is the two halves of that
/// sentence happening in the right order: the value of X is announced at
/// CR 601.2b and the target is named afterwards at CR 601.2c, so the four
/// Swamps' black is still in the pool while the target question stands and
/// only leaves it at CR 601.2h. X = 3 on a printed 1/1 Elf reads (4, 1) and
/// nothing else would: (1, 1) means the pump never landed, and a touched
/// toughness means the +0 was read as a +X. The second Elf across the table is
/// the control — "target creature" is not "every creature" — and walking to
/// the opponent's main phase reads the "until end of turn" off the very
/// creature that had been pumped.
#[test]
fn bloodcurdling_scream_pumps_the_target_it_names_for_the_x_it_was_given() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        // Two creatures on the other side: the scream is aimed at one of
        // them, and the other is the control. Neither belongs to the seat
        // that taps, so anything that taps leaves both of them alone.
        .battlefield(1, &[quiet_creature(), quiet_creature()])
        .hand(0, &[bloodcurdling_scream()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p1, quiet_creature());
    assert_eq!(elves.len(), 2, "two creatures to choose between");
    let (victim, bystander) = (elves[0], elves[1]);
    assert_eq!(pt(&engine, victim), (1, 1), "a printed 1/1 before the pump");

    // Four Swamps, and only the Swamps: no creature of p0's is on this board,
    // so the pool is exactly four black and the {X}{B} can come from nowhere
    // else.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Swamps, and nothing else on this board makes mana"
    );

    cast_with_floating(&mut engine, p0, bloodcurdling_scream());

    // CR 601.2b: the value of X is announced before anything is targeted.
    let Pending::ChooseNumber {
        player, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("an {{X}} spell asks for its X, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the casting seat names its own X");
    assert!(
        (min..=max).contains(&3),
        "four black pays for X = 3 and the {{B}}: the question offered {min}..={max}"
    );
    engine
        .apply(p0, PlayerAction::ChooseNumber(3))
        .expect("three was within the range the question offered");

    // CR 601.2c: and only then is the target chosen.
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("the scream targets a creature, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the casting seat aims it");
    assert!(
        options.contains(&victim) && options.contains(&bystander),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "CR 601.2h pays last: the mana is still floating while the question stands"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("the creature the question offered was chosen");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}} and the {{B}} both came out of the four Swamps' pool"
    );
    assert!(!stack_is_empty(&engine), "and the spell is on the stack");

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, victim),
        (4, 1),
        "+X/+0 with X = 3: power up three, toughness untouched"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the creature the scream did not name is still the 1/1 it was printed as"
    );

    // "until end of turn", read off the same creature once the turn that cast
    // the spell is over.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, victim),
        (1, 1),
        "the pump is gone with the turn that made it"
    );
}
