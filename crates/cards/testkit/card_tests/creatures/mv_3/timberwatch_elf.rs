//! `cards/creatures/mv_3/timberwatch_elf.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Timberwatch Elf is a 1/2 Elf whose whole text is one line: "{T}: Target
/// creature gets +X/+X until end of turn, where X is the number of Elves on
/// the battlefield." The board makes four readings disagree, because the Elf
/// itself, one Llanowar Elves on each side of the table and a Bird beside them
/// are all on the battlefield at once: `(4, 4)` on a targeted printed 1/1 is
/// "every Elf on the battlefield, the source included", where "Elves you
/// control" would be `(3, 3)`, "creatures" `(5, 5)` and a source excluded
/// `(2, 2)`. The {T} and nothing else is the price, so the Forests that cast
/// the Elf are read only as the board the count happens on.
#[test]
fn timberwatch_elf_pumps_by_every_elf_on_the_battlefield_itself_included() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        // An Elf on the other side of the table, and a Bird that is a
        // creature and no Elf: one of them has to count and the other must not.
        .battlefield(1, &[llanowar_elves(), baleful_strix()])
        .hand(0, &[timberwatch_elf()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The Elf arrives for real: {2}{G} out of the three Forests. The ability
    // costs no mana at all, so what the cast leaves in the pool says nothing
    // about the activation below.
    cast_from_hand(&mut engine, p0, timberwatch_elf());
    pass_until(&mut engine, stack_is_empty);
    let timberwatch =
        on_battlefield(&engine, p0, timberwatch_elf()).expect("the Elf resolved onto the table");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let strix = on_battlefield(&engine, p1, baleful_strix()).expect("their Bird is out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");
    assert!(
        !is_tapped(&engine, timberwatch),
        "and the source is untapped"
    );

    // A creature cast this turn cannot pay a {T} (CR 302.6), so the ability
    // is read one turn cycle later — the same rule that keeps it out of a
    // combat, asked of a cost instead of an attack.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");

    // Ability 0 is the one line the card prints, and its whole price is the
    // tap symbol: nothing has to be floating for it to be offered.
    activate(&mut engine, p0, timberwatch_elf(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert!(
        options.contains(&mine) && options.contains(&theirs) && options.contains(&strix),
        "\"target creature\" is any creature on either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("my own Elves were one of the options it enumerated");

    // CR 601.2c names the target and CR 601.2h pays afterwards, so the tap is
    // read here rather than before the answer.
    assert!(is_tapped(&engine, timberwatch), "{{T}} is the whole price");
    assert!(!stack_is_empty(&engine), "and it is no mana ability");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, mine),
        (4, 4),
        "three Elves on the battlefield — the source itself, my Llanowar Elves \
         and the one across the table — and neither the Bird beside them nor a \
         land is one"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump reaches the creature it targeted and no other"
    );
    assert_eq!(
        pt(&engine, strix),
        (1, 1),
        "and a creature that is no Elf is neither counted nor pumped"
    );
}
