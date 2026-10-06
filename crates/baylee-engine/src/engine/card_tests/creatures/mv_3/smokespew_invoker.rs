//! `cards/creatures/mv_3/smokespew_invoker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Smokespew Invoker is a {2}{B} 3/1 Zombie Mutant whose entire rules text is
/// "{7}{B}: Target creature gets -3/-3 until end of turn."
///
/// Both numbers are the engine's answers rather than the card's, so the board
/// is built to make each one legible in a different place. Eleven Swamps tapped
/// in one go pay the {2}{B} and leave exactly the eight the ability charges,
/// read off the pool before the claim — `legal.abilities` is filtered through
/// `can_afford`, and that reads the pool and not the untapped lands. The pump
/// is then measured as a three-point drop on a creature that survives it, with
/// the Elf under the same seat as the control that says the -3/-3 landed only
/// where it was aimed.
#[test]
fn smokespew_invoker_pays_eight_for_minus_three_on_the_creature_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    // Eleven Swamps: enough to cast the Invoker for {2}{B} and still hold the
    // whole {7}{B} the ability asks for, since a pool survives the step it was
    // filled in (CR 500.5) and this scenario never leaves the main phase.
    let mut board = vec![swamp(); 11];
    board.push(quiet_creature());
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &board)
        // The creature "target creature" has to reach, and reaching it must not
        // be a creature of yours by accident.
        .battlefield(1, &[rootbreaker_wurm()])
        .hand(0, &[smokespew_invoker()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf is out");
    let wurm = on_battlefield(&engine, p1, rootbreaker_wurm()).expect("the Wurm is out");

    // The Elf is named as the one source kept back: it prints its own
    // `{T}: Add {G}`, so `tap_all_mana` would have spent it and left a green
    // mana in a pool every assertion below reads as black and nothing else.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        11,
        "eleven Swamps tapped, and the Elf left standing"
    );

    cast_with_floating(&mut engine, p0, smokespew_invoker());
    pass_until(&mut engine, stack_is_empty);
    let invoker = on_battlefield(&engine, p0, smokespew_invoker()).expect("the Invoker resolved");
    assert_eq!(pt(&engine, invoker), (3, 1), "the printed 3/1 body arrived");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "{{2}}{{B}} is spent and exactly the {{7}}{{B}} the ability charges is still floating"
    );

    let before = pt(&engine, wurm);
    assert!(
        before.1 > 3,
        "the target has to survive the pump for its body to be read back: {before:?}"
    );

    // Ability 0 is the only line the card prints, and it is pressed with the
    // mana already floating: the offer is read off the pool.
    activate(&mut engine, p0, smokespew_invoker(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&wurm) && options.contains(&elf),
        "\"target creature\" reaches across the table and does not stop at your own: {options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays last, so the mana is
    // still in the pool and the ability nowhere near the stack.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "the cost is the last step of the activation"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .expect("the Wurm was one of the options the question enumerated");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{7}}{{B}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "and the pump is waiting on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, wurm),
        (before.0 - 3, before.1 - 3),
        "-3/-3 until end of turn on the creature the ability named"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "the creature it did not name is untouched"
    );
}
