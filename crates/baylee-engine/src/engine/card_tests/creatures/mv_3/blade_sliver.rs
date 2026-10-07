//! `cards/creatures/mv_3/blade_sliver.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Blade Sliver is a 2/2 Sliver whose entire text is „All Sliver creatures
/// get +1/+0" is, and the word the card hinges on is *all* and not
/// *you control*: therefore a Blade Sliver stands on **both** sides of the
/// table and the second pumps the first as soon as it arrives — a static that
/// reads only its own Slivers would leave the opposing one at (3, 2).
/// The Llanowar Elf next to one's own Sliver is the downward countercheck:
/// +1/+0 on a creature without the subtype would mean the filter is
/// `Filter::CREATURE` or `YOUR_CREATURE` instead of the printed subtype
/// check. (4, 2) in the end is the only number that reads both statics on
/// both Slivers simultaneously, and (1, 1) on the Elf says that exactly
/// two Slivers and one creature are on the battlefield.
#[test]
fn blade_sliver_pumps_every_sliver_at_the_table_and_no_other_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), llanowar_elves()])
        .battlefield(1, &[blade_sliver()])
        .hand(0, &[blade_sliver()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    let theirs = on_battlefield(&engine, p1, blade_sliver()).expect("their Sliver is out");
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "an Elf is not a Sliver and is not pumped by anything"
    );
    assert_eq!(
        pt(&engine, theirs),
        (3, 2),
        "the one Sliver on the table pumps itself: 2/2 plus its own +1/+0"
    );

    // {2}{R} off three Mountains and the Elf that contributes its own mana
    // ability — none of it is needed afterward, because this test does not
    // read a combat phase, but only projections.
    cast_from_hand(&mut engine, p0, blade_sliver());
    pass_until(&mut engine, stack_is_empty);
    let mine = on_battlefield(&engine, p0, blade_sliver()).expect("the Sliver resolved");

    assert_eq!(
        pt(&engine, mine),
        (4, 2),
        "2/2 gedruckt, +1/+0 vom eigenen Static und +1/+0 vom Sliver gegenüber"
    );
    assert_eq!(
        pt(&engine, theirs),
        (4, 2),
        "\"All Sliver creatures\" — the opponent's Sliver grew when mine arrived, \
         so the static reads the subtype and not \"Slivers you control\""
    );
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "and the Elf under the same control gets nothing: it is the subtype that \
         counts and not `Filter::CREATURE`"
    );
}
