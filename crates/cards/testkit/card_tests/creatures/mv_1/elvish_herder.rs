//! `cards/creatures/mv_1/elvish_herder.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Elvish Herder prints a 1/1 Elf for `{G}` and one line: "`{G}`: Target
/// creature gains trample until end of turn." Both halves are played here,
/// and the two Elves beside it are what make the targeting readable:
/// `Filter::CREATURE` names no controller, so the creature across the table
/// must be on the menu — a menu without it would be a "target creature *you*
/// control" the card does not print. The green is tapped into the pool
/// *before* the offer is claimed, because `legal.abilities` is filtered
/// through `can_afford`, which reads the pool and not the untapped Forest,
/// and the leftover `{G}` from casting the Herder is exactly the pump's
/// price — asserted while the target question is still open (CR 601.2c
/// before 601.2h) and again after it is answered.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn elvish_herder_sells_trample_for_the_green_it_left_floating() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[elvish_herder()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::TRAMPLE),
        "a printed 1/1 Elf has no trample until the Herder says so"
    );

    // Two Forests, with the Elf named as the one thing kept back: it prints a
    // mana ability of its own and would otherwise be counted as a third
    // green that the pool does not have.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, elvish_herder());
    pass_until(&mut engine, stack_is_empty);
    let herder = on_battlefield(&engine, p0, elvish_herder()).expect("the Herder resolved");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "{{G}} paid for the body and the second {{G}} is still floating"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(herder, 0)),
        "the {{G}} is already floating, so the one line the Herder prints is \
         offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, elvish_herder(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that chooses");
    assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" names no controller, so the offer reaches both \
         sides of the table: {options:?}"
    );
    assert!(
        options.contains(&herder),
        "and the Herder is a creature itself: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "CR 601.2h is the last step: the {{G}} is still in the pool while the \
         target is being named"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("a creature the question offered");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and is gone the moment the cost is paid"
    );
    assert!(!stack_is_empty(&engine), "the pump is no mana ability");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, mine).contains(KeywordSet::TRAMPLE),
        "the creature the ability named gained trample"
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "`{{G}}` with 0/0 in it is a keyword grant and not a pump"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::TRAMPLE),
        "the creature it did not name is untouched"
    );
    assert!(
        !keywords(&engine, herder).contains(KeywordSet::TRAMPLE),
        "and the Herder keeps nothing for itself"
    );
    assert!(
        !is_tapped(&engine, herder),
        "the price was mana, not the Herder's own {{T}}"
    );
}
