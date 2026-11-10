//! `cards/enchantments/mv_2/flowstone_surge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Flowstone Surge — {1}{R} Enchantment: "Creatures you control get +1/-1."
///
/// Two readings have to come off one board, because each is the half the other
/// cannot see. A printed 1/1 of mine is buried the moment the Surge resolves —
/// a 2/0 is lethal by CR 704.5f and nothing but the `-1` can have done it —
/// while the Elf across the table is still a 1/1, which is the only way to tell
/// `Filter::YOUR_CREATURE` from every creature in the game. The `+1` needs a
/// creature that survives the subtraction, so a second creature of mine is read
/// before and after and has to move by exactly one in each direction.
#[test]
fn flowstone_surge_gives_plus_one_minus_one_to_your_creatures_only() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                drannith_magistrate(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[flowstone_surge()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let survivor = on_battlefield(&engine, p0, drannith_magistrate()).expect("a creature of mine");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the Surge");
    let before = pt(&engine, survivor);

    // The {1}{R} comes off the two Mountains and the Surge has to actually
    // arrive: a static read off the card file says nothing about which
    // creatures the layer system handed it.
    cast_from_hand(&mut engine, p0, flowstone_surge());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, flowstone_surge()).is_some(),
        "the enchantment resolved onto the table"
    );

    assert_eq!(
        pt(&engine, survivor),
        (before.0 + 1, before.1 - 1),
        "+1/-1 and not a pump of one half: {before:?} becomes two numbers, \
         each moved by exactly one"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "a printed 1/1 under +1/-1 is a 2/0, and CR 704.5f buries it"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "and it is in its owner's graveyard, not merely gone"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "\"creatures you control\": the Elf across the table is untouched"
    );
}
