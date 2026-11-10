//! `cards/creatures/mv_4/furnace_whelp.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Furnace Whelp is a 2/2 Dragon with flying and `Coverage::Implemented` that prints a firebreathing pump.
/// Paying {R} activates its ability to give itself +1/+0 until end of turn.
/// Activating the ability multiple times cumulatively increases its power while leaving toughness and bystanders untouched.
/// Its printed flying keyword is projected through the layer system onto the battlefield permanent.
#[test]
fn furnace_whelp_has_flying_and_pumps_power_with_firebreathing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[furnace_whelp(), mountain(), mountain()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let whelp =
        on_battlefield(&engine, p0, furnace_whelp()).expect("Furnace Whelp is on battlefield");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    assert_eq!(pt(&engine, whelp), (2, 2), "printed body is 2/2");
    assert!(
        keywords(&engine, whelp).contains(KeywordSet::FLYING),
        "Furnace Whelp has flying"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Mountains provide two red mana"
    );

    activate(&mut engine, p0, furnace_whelp(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, whelp),
        (3, 2),
        "first activation gave +1/+0, body is now 3/2"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "bystander creature power and toughness remain unchanged"
    );

    activate(&mut engine, p0, furnace_whelp(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, whelp),
        (4, 2),
        "second activation gave another +1/+0, body is now 4/2"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "both floating red mana were spent on activations"
    );
}
