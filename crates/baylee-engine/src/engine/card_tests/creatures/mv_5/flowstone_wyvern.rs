//! `cards/creatures/mv_5/flowstone_wyvern.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Flowstone Wyvern prints flying and one activated line: "{R}: This creature
/// gets +2/-2 until end of turn" — a pump whose two halves disagree, which is
/// the whole card. Seven Mountains pay the {3}{R}{R} and leave exactly the two
/// red the ability charges, so one activation has to read (5, 1) on a printed
/// 3/3 while the Elf across the table stays a printed 1/1: `Filter::This` is
/// narrowed to the source, and the toughness is read as well as the power. The
/// second activation is the other half of the same sentence — 7/-1 is no body
/// at all, and CR 704.5f puts it in the graveyard, which a line misread as
/// "+2/+2" could never do.
#[test]
fn flowstone_wyvern_pumps_the_power_it_prints_and_pays_the_toughness_for_it() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 7])
        .hand(0, &[flowstone_wyvern()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Seven Mountains: {3}{R}{R} brings the Drake to the table and leaves
    // exactly the two red the one printed line charges, in the same main phase
    // (CR 500.5).
    cast_from_hand(&mut engine, p0, flowstone_wyvern());
    pass_until(&mut engine, stack_is_empty);

    let wyvern = on_battlefield(&engine, p0, flowstone_wyvern()).expect("the Drake resolved");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, wyvern), (3, 3), "the body the card prints");
    assert!(
        keywords(&engine, wyvern).contains(KeywordSet::FLYING),
        "and the flying it prints beside it"
    );
    assert_eq!(pt(&engine, theirs), (1, 1), "a printed 1/1 as the control");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "seven Mountains less the {{3}}{{R}}{{R}} the Drake cost"
    );

    // Ability 0 is the printed "{R}: This creature gets +2/-2 until end of turn."
    activate(&mut engine, p0, flowstone_wyvern(), 0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{R}} came out of a pool only the Mountains filled"
    );
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so the ability is waiting on the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, wyvern),
        (5, 1),
        "+2/-2 on a printed 3/3 — both halves of the sentence, and not the \\
         power alone"
    );
    assert!(
        keywords(&engine, wyvern).contains(KeywordSet::FLYING),
        "the pump grants no keyword and takes none away"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "`Filter::This` reaches its own source and never across the table"
    );

    // The same line once more: 3/3 plus two of these is a 7/-1, which is not a
    // permanent at all.
    activate(&mut engine, p0, flowstone_wyvern(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the second {{R}} came out of the pool"
    );
    assert!(
        on_battlefield(&engine, p0, flowstone_wyvern()).is_none(),
        "a creature with toughness 0 or less goes to the graveyard (CR 704.5f)"
    );
    assert!(
        in_graveyard(&engine, p0, flowstone_wyvern()).is_some(),
        "and it is in its owner's graveyard rather than merely off the battlefield"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the Elf that was never the source never moved"
    );
}
