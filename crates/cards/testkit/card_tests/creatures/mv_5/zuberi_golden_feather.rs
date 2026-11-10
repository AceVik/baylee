//! `cards/creatures/mv_5/zuberi_golden_feather.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Zuberi, Golden Feather — `{4}{W}`, a legendary 3/3 Griffin with flying
/// and "Other Griffin creatures get +1/+1".
///
/// A lone Zuberi would prove nothing, since a card with no static at all
/// leaves it the 3/3 it prints. So the second Zuberi sits across the table and
/// is read twice: a lone Griffin at 3/3 before, a 4/4 the moment the cast one
/// resolves — that difference is the pump and not a body. The Elf beside them
/// is no Griffin, and the cast Zuberi's own 4/4 is the other half of "other":
/// 5/5 would be a lord that had pumped itself as well.
#[test]
fn zuberi_pumps_the_other_griffin_across_the_table_and_never_itself() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[zuberi_golden_feather()])
        // The other Griffin, and the only one there is: the printed line
        // excludes the source itself and says nothing about who controls the
        // rest, so this is exactly the creature the static is about.
        .battlefield(1, &[zuberi_golden_feather()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let theirs = on_battlefield(&engine, p1, zuberi_golden_feather()).expect("their Zuberi is out");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    assert_eq!(
        pt(&engine, theirs),
        (3, 3),
        "the printed body: no other Griffin stands on the battlefield yet"
    );
    assert_eq!(pt(&engine, elf), (1, 1), "and a printed 1/1 beside it");

    // Five Plains pay the {4}{W}. The Elf is named as the printing kept back:
    // it is the board's non-Griffin control, and a creature tapped for mana
    // would read as a different board.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five tapped Plains and an untapped Elf: five mana"
    );
    cast_with_floating(&mut engine, p0, zuberi_golden_feather());
    pass_until(&mut engine, stack_is_empty);

    let mine = on_battlefield(&engine, p0, zuberi_golden_feather()).expect("the Zuberi resolved");
    assert!(
        keywords(&engine, mine).contains(KeywordSet::FLYING),
        "the printed flying reaches the permanent"
    );
    assert_eq!(
        pt(&engine, theirs),
        (4, 4),
        "the Griffin that was already standing is a 4/4 now: \"other Griffin \
         creatures get +1/+1\" is the arrived one's static, not a body"
    );
    assert_eq!(
        pt(&engine, mine),
        (4, 4),
        "+1/+1 from the Griffin across the table and none from itself — a \
         static without Filter::Another would have read 5/5 off the pair"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "the Elf is no Griffin, so nothing on this table pumps it"
    );
}
