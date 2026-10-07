//! `cards/creatures/mv_3/balthor_the_stout.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Balthor the Stout — {1}{R}{R} legendary 2/2 Dwarf Barbarian — prints
/// "Other Barbarian creatures get +1/+1" and "{R}: Another target Barbarian
/// creature gets +1/+0 until end of turn". Neither sentence says "you
/// control", so the board puts a Barbarian on each side of the table: each
/// one is a 3/3 because the *other* one hands it +1/+1, and the red
/// activation aimed across the table reads 4/3 rather than 3/3. The Elf
/// beside the opponent's Balthor is the counter-half of both words — it is
/// no Barbarian, so it is neither pumped nor offered as a target.
#[test]
fn balthor_the_stout_pumps_the_other_barbarian_and_never_himself() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[balthor_the_stout(), mountain()])
        .battlefield(1, &[balthor_the_stout(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, balthor_the_stout()).expect("my Balthor is out");
    let theirs = on_battlefield(&engine, p1, balthor_the_stout()).expect("their Balthor is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    // "Other Barbarian creatures": each Balthor is the printed 2/2 plus the
    // +1/+1 the other one hands it, because the line names no controller.
    assert_eq!(pt(&engine, mine), (3, 3), "my Balthor, pumped by theirs");
    assert_eq!(pt(&engine, theirs), (3, 3), "and theirs, pumped by mine");
    assert_eq!(pt(&engine, elf), (1, 1), "the Elf is no Barbarian");

    // `can_afford` reads the pool and not the untapped land, so the red is
    // floating before anything is claimed about the offer.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one Mountain, and Balthor makes no mana of his own"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(mine, 1)),
        "ability 1 is the {{R}} pump, offered now that the red is in the pool: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, balthor_the_stout(), 1);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("the pump targets a creature, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat chooses");
    assert!(
        options.contains(&theirs),
        "\"another target Barbarian creature\" reaches across the table: {options:?}"
    );
    assert!(
        !options.contains(&mine),
        "\"another\" is not the Balthor paying the red: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "the Elf beside them is no Barbarian: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the creature the question offered is the one it named");
    assert!(
        !stack_is_empty(&engine),
        "the pump is no mana ability, so it is waiting on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, theirs),
        (4, 3),
        "+1/+0 on top of the +1/+1 it was already handed by the other Balthor"
    );
    assert_eq!(
        pt(&engine, mine),
        (3, 3),
        "and the Balthor that paid is untouched"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{R}} came out of the pool"
    );
}
