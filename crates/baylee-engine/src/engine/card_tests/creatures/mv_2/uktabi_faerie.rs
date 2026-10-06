//! `cards/creatures/mv_2/uktabi_faerie.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Uktabi Faerie is a 1/1 flier whose whole text is "{3}{G}, Sacrifice this
/// creature: Destroy target artifact."
///
/// The scenario reads all three parts of that price and the one word that
/// decides what the ability may point at: the four Forests are tapped before
/// anything is claimed, because `legal.abilities` is filtered through
/// `can_afford` and reads the pool rather than the untapped lands; the target
/// question is answered while the Faerie is still on the battlefield and the
/// mana still floating (CR 601.2h pays last); and the artifact across the
/// table stands beside an Elf, so "target artifact" is read and not skipped.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn uktabi_faerie_sacrifices_itself_to_destroy_a_target_artifact() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), uktabi_faerie()],
        )
        .battlefield(1, &[quiet_artifact(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let faerie = on_battlefield(&engine, p0, uktabi_faerie()).expect("the Faerie is out");
    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, faerie), (1, 1), "a printed 1/1");
    assert!(
        keywords(&engine, faerie).contains(KeywordSet::FLYING),
        "with the flying the card prints"
    );

    // The price is `{3}{G}`, and the pool is where the engine reads it. The
    // Faerie prints no mana ability of its own, so the four Forests are the
    // whole of the board's mana and the whole of the count.
    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(taken, 4, "the four Forests and nothing else on this board");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four green floating out of the four Forests"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(faerie, 0)),
        "with {{3}}{{G}} in the pool the Faerie's one line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, uktabi_faerie(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target artifact\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&rock),
        "the artifact across the table is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&elves),
        "the Elf is a creature and no artifact: {options:?}"
    );
    assert!(
        !options.contains(&faerie),
        "the Faerie is the price, not a target: {options:?}"
    );
    assert_eq!(options, vec![rock], "the one artifact in the game");

    // CR 601.2h: the sacrifice and the mana are the last step of the
    // activation, so both are still here while the question stands.
    assert!(
        on_battlefield(&engine, p0, uktabi_faerie()).is_some(),
        "nothing is paid before the target is named"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "and the {{3}}{{G}} is still floating"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rock],
            },
        )
        .expect("the artifact the question offered was chosen");

    assert!(
        on_battlefield(&engine, p0, uktabi_faerie()).is_none(),
        "\"Sacrifice this creature\" takes the Faerie itself"
    );
    assert!(
        in_graveyard(&engine, p0, uktabi_faerie()).is_some(),
        "and a sacrificed creature goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}}{{G}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "destroying an artifact is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "the artifact that was named left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "and it is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "while the Elf the ability did not name never moved"
    );
}
