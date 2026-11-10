//! `cards/sorceries/mv_2/wielding_the_green_dragon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wielding the Green Dragon costs {1}{G} and prints one sentence: "Target
/// creature gets +4/+4 until end of turn." The creature that is chosen is the
/// **opponent's** Elf, because `Filter::CREATURE` reaches across the table and
/// a pump quietly narrowed to "you control" would never have offered it — while
/// the printed 1/1 under the caster's own control is the control that says the
/// +4/+4 landed on the creature the spell named and not on the whole board.
/// `(5, 5)` is the only body that reads both printed numbers: a `(5, 1)` would
/// mean the toughness half was dropped, and a surviving 1/1 anywhere means the
/// effect was a board buff.
#[test]
fn wielding_the_green_dragon_pumps_only_the_creature_it_targets() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), llanowar_elves()])
        .hand(0, &[wielding_the_green_dragon()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");
    assert_eq!(pt(&engine, theirs), (1, 1), "and one across the table");

    cast_from_hand(&mut engine, p0, wielding_the_green_dragon());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster aims it");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and those two creatures are the whole menu: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the creature the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, theirs),
        (5, 5),
        "+4/+4 on the creature the spell named"
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "and nothing at all for the creature it did not target — a pump that \
         had reached every creature on the board would leave this at (5, 5)"
    );
}
