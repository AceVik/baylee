//! `cards/enchantments/mv_4/day_of_destiny.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Day of Destiny prints one sentence — "Legendary creatures you control get
/// +2/+2" — and three of its words each need their own witness on the same
/// board: Katara, the Fearless under my control is the legendary creature the
/// static is about, a Llanowar Elves beside her is the creature "legendary"
/// has to decline, and the very same card across the table is what tells "you
/// control" from "legendary creatures". The enchantment is *cast* rather than
/// seated, so the pump is read off a board it actually arrived on: a printing
/// that had never resolved would leave all three at their printed bodies.
#[test]
fn day_of_destiny_pumps_the_legendary_creatures_you_control_and_no_other() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                katara_the_fearless(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[katara_the_fearless()])
        .hand(0, &[day_of_destiny()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, katara_the_fearless()).expect("my legend is out");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    let theirs = on_battlefield(&engine, p1, katara_the_fearless()).expect("their legend is out");
    assert_eq!(
        pt(&engine, mine),
        (3, 3),
        "a printed 3/3 before the enchantment"
    );
    assert_eq!(pt(&engine, elf), (1, 1), "and a printed 1/1 beside her");
    assert_eq!(
        pt(&engine, theirs),
        (3, 3),
        "the same printing stands across the table at the same body"
    );

    // Four Plains pay the {3}{W} the enchanment costs; the pump is only worth
    // reading once the static is on the battlefield.
    cast_from_hand(&mut engine, p0, day_of_destiny());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, day_of_destiny()).is_some(),
        "the enchantment resolved onto the battlefield"
    );

    assert_eq!(
        pt(&engine, mine),
        (5, 5),
        "\"Legendary creatures you control get +2/+2\" — both halves of the \
         pump, on the creature that is legendary and mine"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "\"legendary\" is read and not skipped: the Elf beside her is a \
         creature I control and no legend"
    );
    assert_eq!(
        pt(&engine, theirs),
        (3, 3),
        "\"you control\" is the other word: the same card across the table is \
         legendary and untouched"
    );
}
