//! `cards/creatures/mv_2/ironshell_beetle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ironshell Beetle prints one sentence — "When this creature enters, put a
/// +1/+1 counter on target creature" — and the word carrying it is "target":
/// the offer has to reach across the table, not just the Beetle's own board.
/// So an Elf stands on each side, and the counter is read off the creature the
/// trigger *named* (a 2/2 against the untouched printed 1/1), because a trigger
/// that resolved against nothing would leave both bodies exactly as printed.
#[test]
fn ironshell_beetle_puts_its_counter_on_the_creature_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[ironshell_beetle()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "a printed 1/1 before the trigger"
    );

    // {1}{G} off the board. The helper spends both Elves for mana on the way,
    // which costs this scenario nothing: a tapped creature is still on the
    // battlefield and still a legal thing to name.
    cast_from_hand(&mut engine, p0, ironshell_beetle());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until only stops on a target choice")
    };
    assert_eq!(player, p0, "the Beetle's controller chooses");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" reaches either side of the table: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Elf across the table was one of the options");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, theirs),
        (2, 2),
        "the +1/+1 counter landed on the creature the trigger named"
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "and on nothing else: the Elf nobody named is the 1/1 it was printed as"
    );
    assert!(
        on_battlefield(&engine, p0, ironshell_beetle()).is_some(),
        "the Beetle itself resolved and stayed, its trigger already spent"
    );
}
