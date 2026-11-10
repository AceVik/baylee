//! `cards/lands/utility/okina_temple_to_the_grandfathers.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Okina, Temple to the Grandfathers is a legendary land under `Coverage::Implemented` that taps for {G} or pumps a legendary creature.
/// Activating its second ability for {G}, {T} targets a legendary creature and grants +1/+1 until end of turn.
/// Non-legendary creatures are excluded from the target selection.
/// Upon resolution, the legendary creature grows while bystanders and mana pool are updated accordingly.
#[test]
fn okina_temple_to_the_grandfathers_pumps_target_legendary_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                okina_temple_to_the_grandfathers(),
                forest(),
                thorin_oakenshield(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let okina = on_battlefield(&engine, p0, okina_temple_to_the_grandfathers())
        .expect("Okina is on battlefield");
    let thorin =
        on_battlefield(&engine, p0, thorin_oakenshield()).expect("Thorin is on battlefield");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Elf is on battlefield");

    assert_eq!(pt(&engine, thorin), (3, 2), "Thorin prints a 3/2");
    assert_eq!(pt(&engine, elf), (1, 1), "Elf is a 1/1");

    // The Forest alone: Okina pays its own {T} below, and the Elves' printed
    // `{T}: Add {G}` would be a second green this test never accounted for.
    tap_mana_where(&mut engine, p0, |id| id != okina && id != elf);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one Forest tapped for green mana, leaving Okina untapped"
    );

    activate(&mut engine, p0, okina_temple_to_the_grandfathers(), 1);

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "activating player chooses target");
    assert_eq!((min, max), (1, 1), "one target required");
    assert!(
        options.contains(&thorin),
        "legendary creature is an offered target: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "non-legendary creature is excluded from target selection: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![thorin],
            },
        )
        .expect("targeting Thorin is legal");

    assert!(
        is_tapped(&engine, okina),
        "Okina tapped to pay the activation cost"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, thorin),
        (4, 3),
        "Thorin gained +1/+1, becoming 4/3"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "non-legendary creature is untouched"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "floating green mana was spent"
    );
}
