//! `cards/enchantments/mv_3/dralnu_s_crusade.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dralnu's Crusade prints three sentences that all share one filter: all
/// Goblins get +1/+1, all Goblins are black, and all Goblins are Zombies in
/// addition to their other creature types. The pump is the clause this board
/// can differ on — Festering Goblin already prints Zombie Goblin and is
/// already black — so it is read on **both** sides of the table, because "All
/// Goblins" is not "Goblins you control". The Skyclave Apparition beside them
/// is the filter's edge: a creature the same seat controls and no Goblin, so
/// a filter that had lost `Filter::HasSubtype` would have pumped it to 3/3.
#[test]
fn dralnu_s_crusade_pumps_the_goblins_on_both_sides_of_the_table_and_no_other_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, forest())
        .battlefield(
            0,
            &[
                swamp(),
                mountain(),
                forest(),
                festering_goblin(),
                skyclave_apparition(),
            ],
        )
        .battlefield(1, &[festering_goblin()])
        .hand(0, &[dralnu_s_crusade()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, festering_goblin()).expect("my Goblin stands");
    let theirs = on_battlefield(&engine, p1, festering_goblin()).expect("their Goblin stands");
    let other = on_battlefield(&engine, p0, skyclave_apparition()).expect("the Apparition stands");
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "a printed 1/1 before the Crusade"
    );
    assert_eq!(
        pt(&engine, other),
        (2, 2),
        "and a printed 2/2 that is no Goblin"
    );

    // {1}{B}{R} out of the Swamp, the Mountain and the Forest, which is every
    // mana source on this board.
    cast_from_hand(&mut engine, p0, dralnu_s_crusade());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, dralnu_s_crusade()).is_some(),
        "the Crusade resolved onto the table"
    );

    assert_eq!(
        pt(&engine, mine),
        (2, 2),
        "\"All Goblins get +1/+1\", on the Goblin under the Crusade's controller"
    );
    assert_eq!(
        pt(&engine, theirs),
        (2, 2),
        "and on the Goblin across the table: the card says all Goblins, not all \
         of yours"
    );
    assert_eq!(
        pt(&engine, other),
        (2, 2),
        "a creature that is no Goblin is untouched — a filter that had lost its \
         subtype would have pumped this one too"
    );
}
