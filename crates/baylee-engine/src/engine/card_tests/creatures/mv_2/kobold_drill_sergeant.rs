//! `cards/creatures/mv_2/kobold_drill_sergeant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Kobold Drill Sergeant — {1}{R}, a 1/2 Kobold Soldier whose whole text is
/// one static: "Other Kobold creatures you control get +0/+1 and have
/// trample."
///
/// The board carries a Sergeant on each side of the table, an Elf under the
/// seat that will cast a second one, and that second one in hand, because
/// every word of the filter needs its own witness: the lone Sergeant reads
/// the printed (1, 2) with no trample, which is the word "Another" being read
/// rather than skipped; the Elf stays a (1, 1), which is "Kobold"; and the
/// Sergeant across the table stays (1, 2), which is "you control".
///
/// Casting the second one is the whole card in one move — it arrives as an
/// *other* Kobold to the one beside it, so the pair reads (1, 3) with trample
/// where a moment earlier nothing on either side moved.
#[test]
fn kobold_drill_sergeant_arms_only_the_other_kobolds_its_controller_has() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                llanowar_elves(),
                kobold_drill_sergeant(),
            ],
        )
        .battlefield(1, &[kobold_drill_sergeant()])
        .hand(0, &[kobold_drill_sergeant()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let sergeant =
        on_battlefield(&engine, p0, kobold_drill_sergeant()).expect("a Sergeant is on the table");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    let theirs =
        on_battlefield(&engine, p1, kobold_drill_sergeant()).expect("their Sergeant is out");

    assert_eq!(
        pt(&engine, sergeant),
        (1, 2),
        "the only Kobold under this seat is not an *other* Kobold to itself"
    );
    assert!(
        !keywords(&engine, sergeant).contains(KeywordSet::TRAMPLE),
        "and the static does not grant its own source the keyword it prints"
    );
    assert_eq!(pt(&engine, elves), (1, 1), "an Elf is no Kobold");
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::TRAMPLE),
        "so the Elf keeps its printed body and gains nothing"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 2),
        "and a Kobold the other seat controls is not one *you* control"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::TRAMPLE),
        "so the Sergeant across the table never moves"
    );

    // The second one arrives the way a card arrives: {1}{R} off the two
    // Mountains (and the Elf, whose only ability is its own {T}).
    cast_from_hand(&mut engine, p0, kobold_drill_sergeant());
    pass_until(&mut engine, stack_is_empty);

    let pair = all_on_battlefield(&engine, p0, kobold_drill_sergeant());
    assert_eq!(
        pair.len(),
        2,
        "the cast Sergeant resolved beside the seated one"
    );
    for id in pair {
        assert_eq!(
            pt(&engine, id),
            (1, 3),
            "+0/+1 from the other Kobold beside it, and one point of toughness \
             per Sergeant — not two from a static that read itself"
        );
        assert!(
            keywords(&engine, id).contains(KeywordSet::TRAMPLE),
            "and trample off the same static"
        );
    }
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "\"Kobold\" is read and not skipped: the Elf beside them never grew"
    );
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::TRAMPLE),
        "nor did it gain trample"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 2),
        "\"you control\" is read too: their Sergeant counts its own side and \
         not mine"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::TRAMPLE),
        "and it stays the printed 1/2 it entered as"
    );
}
