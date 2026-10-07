//! `cards/creatures/mv_3/kabuto_moth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Kabuto Moth is a `{2}{W}` 1/2 Spirit whose whole printed text is flying and
/// "`{T}`: Target creature gets +1/+2 until end of turn." The board puts a
/// printed 1/1 on each side of the table, because "target creature" is any
/// creature and not "a creature you control" — and the answer is given to the
/// Elf *across* the table, so a pump that had quietly narrowed to the Moth's
/// own side could not satisfy it while the Elf under the same seat reads
/// `(1, 1)`. The `{T}` is read where CR 601.2h puts it: the Moth is still
/// untapped while the target question stands and tapped once it is answered,
/// and getting +1/+2 is no mana ability, so the ability uses the stack. The
/// next main phase is the other half of "until end of turn".
#[test]
fn kabuto_moth_gives_one_power_and_two_toughness_to_the_creature_it_targets() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(17, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
                kabuto_moth(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let moth = on_battlefield(&engine, p0, kabuto_moth()).expect("the Moth is out");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, moth), (1, 2), "the printed 1/2 body");
    assert!(
        keywords(&engine, moth).contains(KeywordSet::FLYING),
        "the printed flying line reaches the permanent"
    );
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");

    // Ability 0 is the only line the card prints, and its whole price is its
    // own tap symbol: the pool is empty and stays empty, which is what says no
    // mana is part of this activation.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the pump costs no mana at all"
    );
    activate(&mut engine, p0, kabuto_moth(), 0);

    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" reaches either side of the table: {options:?}"
    );
    assert!(
        player_options.is_empty(),
        "\"target creature\" names no player (CR 115.4): {player_options:?}"
    );
    // CR 601.2c picks the target and CR 601.2h pays afterwards, so the tap is
    // still owed while this question stands.
    assert!(
        !is_tapped(&engine, moth),
        "{{T}} is the last step of the activation, not the first"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Elf across the table was one of the options");
    assert!(
        is_tapped(&engine, moth),
        "the tap was paid after the target"
    );
    assert!(
        !stack_is_empty(&engine),
        "getting +1/+2 is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, theirs),
        (2, 3),
        "+1/+2 on the creature the Moth named, and the printed 1/1 is what makes \
         both numbers legible"
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "the creature it did not name is untouched, so this is a target and not \
         a board buff"
    );
    assert_eq!(pt(&engine, moth), (1, 2), "nor does the Moth pump itself");

    // "Until end of turn", read from the far side: an effect that had no
    // duration at all would still be sitting there in the next main phase.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the +1/+2 lasted only the turn it was given"
    );
}
