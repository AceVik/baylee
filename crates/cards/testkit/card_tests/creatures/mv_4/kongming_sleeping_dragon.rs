//! `cards/creatures/mv_4/kongming_sleeping_dragon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "21e9e1a9-5d6d-473e-adab-6a1e8e2b0ebd"

// Smuggler's Copter — an artifact printing a 3/3 body that is no creature
// until a crew payment animates it. It is the control the word "creatures"
// needs: a lord whose filter had lost `Filter::CREATURE` would read it as a
// 4/4 while everything else on the board kept passing.
// oracle_id = "49136bdc-bc50-49a2-999a-1ef9c16ea130"

/// Kongming, "Sleeping Dragon" — {2}{W}{W} legendary 2/2 Human Advisor whose
/// whole text is one static: "Other creatures you control get +1/+1."
///
/// Three words of that sentence each need a different witness on one board,
/// and a fourth claim needs a permanent that is no creature at all. The Dragon
/// must stay the 2/2 it prints (*other* creatures), the Llanowar Elves under
/// its controller must read 2/2 (*creatures you control*), the same Elves
/// across the table must stay a printed 1/1, and the uncrewed Smuggler's
/// Copter — an artifact with a printed 3/3 body and no creature type — must
/// stay 3/3, which is what a filter that had lost `Filter::CREATURE` would
/// turn into 4/4. The pumped Elf also carries no +1/+1 counter, so the change
/// is a projection the layers make rather than a counter somebody placed.
#[test]
fn kongming_pumps_every_creature_you_control_but_neither_himself_nor_the_copter() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
                smugglers_copter_for_kongming(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[kongming_sleeping_dragon()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let copter =
        on_battlefield(&engine, p0, smugglers_copter_for_kongming()).expect("the Copter is out");
    assert_eq!(pt(&engine, mine), (1, 1), "no lord is on the board yet");
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and the same card across the table"
    );
    assert_eq!(
        pt(&engine, copter),
        (3, 3),
        "an uncrewed Vehicle is an artifact with a body and no creature type"
    );

    // {2}{W}{W} off the four Plains; the Elves' own tap-for-{G} is a mana
    // route the helper takes on the way, which is a board fact and not the
    // subject of any claim here.
    cast_from_hand(&mut engine, p0, kongming_sleeping_dragon());
    pass_until(&mut engine, stack_is_empty);

    let dragon = on_battlefield(&engine, p0, kongming_sleeping_dragon())
        .expect("the Dragon resolved onto the table");
    assert_eq!(
        pt(&engine, dragon),
        (2, 2),
        "\"Other creatures you control\" — the Dragon is not one of them"
    );
    assert_eq!(
        pt(&engine, mine),
        (2, 2),
        "a printed 1/1 under its controller reads 2/2"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "\"you control\" — the same card across the table never moved"
    );
    assert_eq!(
        pt(&engine, copter),
        (3, 3),
        "\"creatures\" is read: an artifact with a printed body and no creature \
         type stays exactly where the card left it"
    );
    assert_eq!(
        counters_on(&engine, mine, CounterKind::P1P1),
        0,
        "the grant is a static the layers project, not a counter anyone placed"
    );
}
