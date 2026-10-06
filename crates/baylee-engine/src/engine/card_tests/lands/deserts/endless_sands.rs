//! `cards/lands/deserts/endless_sands.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Endless Sands prints one mana ability and two linked ones: `{2}, {T}`
/// exiles **target creature you control** with a link to this land, and
/// `{4}, {T}`, sacrifice it, brings every card exiled with it back under its
/// owner's control. Both prices pay `{T}`, so no one turn can take both —
/// the scenario walks a full turn cycle and lets the untap step stand the
/// land back up, which is also what makes the second payment real instead of
/// a label. The Elf across the table is the control on "you control": the
/// exile may not name it, and it is still standing when the linked return
/// happens.
#[test]
#[allow(clippy::too_many_lines)] // one land, played through every clause it prints
fn endless_sands_exiles_a_creature_with_itself_and_returns_it_when_sacrificed() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9090, forest())
        .battlefield(
            0,
            &[
                endless_sands(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let sands = on_battlefield(&engine, p0, endless_sands()).expect("the Sands are on the table");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let exiled_before = engine.state().zones.list(ZoneLocation::Exile(p0)).len();

    // Six Forests and the Elves' own tap, with the Sands kept back: its {T}
    // is the price both printed abilities are paid with, so tapping it for
    // {C} would be spending the permanent under test.
    tap_all_mana_but(&mut engine, p0, Some(endless_sands()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "six Forests and one Elf, and nothing at all off the Sands"
    );
    assert!(!is_tapped(&engine, sands), "which is still standing");

    // Ability 0 is the mana ability; ability 1 is the exile.
    activate(&mut engine, p0, endless_sands(), 1);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature you control\" is a choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat does the aiming");
    assert!(
        options.contains(&mine),
        "my own Elves are a creature I control: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "and \"you control\" declines the Elf across the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the exiled creature left the battlefield"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Exile(p0)).len(),
        exiled_before + 1,
        "and is exiled with the Sands rather than merely gone"
    );
    assert!(
        is_tapped(&engine, sands),
        "the {{T}} in the price came out of the land itself"
    );

    // Both abilities pay {T}, so the second one waits for the untap step.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 takes another turn with the creature still exiled"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "nothing brought it back on the way round"
    );
    assert!(
        !is_tapped(&engine, sands),
        "the untap step is what lets the land pay {{T}} a second time"
    );

    tap_all_mana_but(&mut engine, p0, Some(endless_sands()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Forests, and the Elf that tapped last turn is in exile"
    );

    // Ability 2: {4}, {T}, sacrifice this land.
    activate(&mut engine, p0, endless_sands(), 2);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, endless_sands()).is_none(),
        "the land sacrificed itself to pay its own last price"
    );
    assert!(
        in_graveyard(&engine, p0, endless_sands()).is_some(),
        "a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Exile(p0)).len(),
        exiled_before,
        "the cards exiled with it are no longer exiled"
    );
    let returned = on_battlefield(&engine, p0, llanowar_elves())
        .expect("the Elf came back under its owner's control");
    assert_eq!(
        pt(&engine, returned),
        (1, 1),
        "as the 1/1 it was printed as"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the Elf across the table never moved"
    );
}
