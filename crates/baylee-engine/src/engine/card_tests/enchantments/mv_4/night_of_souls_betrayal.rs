//! `cards/enchantments/mv_4/night_of_souls_betrayal.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Night of Souls' Betrayal — {2}{B}{B}: "All creatures get -1/-1."
///
/// One sentence, three claims, and no witness supplies two of them: the
/// **same** larger creature on both sides of the table is exactly one point
/// smaller afterwards and still standing — so the modifier is -1/-1, it
/// reaches every creature and not only the caster's, and it is no "destroy all
/// creatures" — a printed 1/1 underneath it dies the moment the enchantment
/// resolves (CR 704.5f), and a second 1/1 cast *afterwards* enters and dies
/// the same way, which a static read off the board only once, as it arrived,
/// would leave standing.
#[test]
fn night_of_souls_betrayal_shrinks_every_creature_by_one_and_buries_the_one_ones() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                forest(),
                thrun_the_last_troll(),
                quiet_creature(),
            ],
        )
        .battlefield(1, &[thrun_the_last_troll()])
        .hand(0, &[night_of_souls_betrayal(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let my_troll = on_battlefield(&engine, p0, thrun_the_last_troll()).expect("my Troll is out");
    let their_troll =
        on_battlefield(&engine, p1, thrun_the_last_troll()).expect("their Troll is out");
    let elves = on_battlefield(&engine, p0, quiet_creature()).expect("the Elves are out");
    let forest_land = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    let (power, toughness) = pt(&engine, my_troll);
    assert_eq!(
        pt(&engine, their_troll),
        (power, toughness),
        "the same card on both sides of the table, before anything is asked of either"
    );
    assert!(
        toughness > 1,
        "the body the -1/-1 is read off has to be one that survives it: {toughness}"
    );
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "and a printed 1/1 is what the modifier has to bury"
    );

    // Four Swamps pay {2}{B}{B} exactly. The Elves and the Forest are named as
    // the two sources kept back: the Elves print a mana ability of their own,
    // which is a route `tap_mana_where` would otherwise take (#159), and the
    // Forest is what the second Elves below is cast with.
    tap_mana_where(&mut engine, p0, |id| id != elves && id != forest_land);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Swamps in the pool, and neither the Elves nor the Forest gave anything"
    );

    cast_with_floating(&mut engine, p0, night_of_souls_betrayal());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, night_of_souls_betrayal()).is_some(),
        "the enchantment resolved onto the battlefield"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{2}}{{B}}{{B}} came out of the pool"
    );

    assert_eq!(
        pt(&engine, my_troll),
        (power - 1, toughness - 1),
        "-1/-1 on the creature this seat owns"
    );
    assert_eq!(
        pt(&engine, their_troll),
        (power - 1, toughness - 1),
        "\"all creatures\" is not \"creatures you control\": the same card across the table lost the same point"
    );
    assert!(
        on_battlefield(&engine, p0, thrun_the_last_troll()).is_some(),
        "and the bigger body survives it — this is no \"destroy all creatures\""
    );
    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_none(),
        "a printed 1/1 with -1/-1 is a 0/0, and CR 704.5f puts it in the graveyard"
    );
    assert_eq!(
        mine(&engine, p0, quiet_creature(), Zone::Graveyard).len(),
        1,
        "which is where it went, rather than merely off the battlefield"
    );

    // The second 1/1 sat in hand while the enchantment arrived, so the static
    // is read for it again: the Forest pays its {G} and it dies the same way.
    cast_from_hand(&mut engine, p0, quiet_creature());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_none(),
        "an Elves cast under the enchantment enters and is buried at once"
    );
    assert_eq!(
        mine(&engine, p0, quiet_creature(), Zone::Graveyard).len(),
        2,
        "both 1/1s are in the graveyard, so the -1/-1 is not a one-shot read at the moment it resolved"
    );
}
