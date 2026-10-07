//! `cards/sorceries/mv_2/monstrous_growth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Monstrous Growth prints one sentence — "{1}{G} sorcery: Target creature
/// gets +4/+4 until end of turn." — and the board makes every word of it
/// load-bearing. "Target creature" reaches either side of the table, so the
/// opponent's Elf is the option list's other half and the pump must land only
/// on the creature that was named; +4/+4 on a printed 1/1 reads (5, 5), which a
/// +4/+0 or a +0/+4 could not produce; and the {1}{G} is a real payment out of
/// a pool two Forests filled, with nothing floating while the target question
/// stands (CR 601.2c before CR 601.2h).
#[test]
fn monstrous_growth_pumps_the_creature_it_targets_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[monstrous_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the spell");

    // Three Forests pay the {2} with the Elves named as the printing kept
    // back: they are the creatures this test reads afterwards, and a host
    // tapped for its own mana reads wrong down the page.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three tapped Forests and neither Elf"
    );

    cast_with_floating(&mut engine, p0, monstrous_growth());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures under my own control are offered: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"target creature\" is not \"creature you control\": the Elf across \
         the table is a legal target too: {options:?}"
    );
    assert!(
        !options.contains(&on_battlefield(&engine, p0, forest()).expect("my Forest is out")),
        "a land is no creature at all: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "CR 601.2h pays **after** CR 601.2c chooses, so the three green are \
         still floating while the target question is open"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, host),
        (5, 5),
        "+4/+4 on the creature that was named"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody targeted is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and the static never reaches across the table to a creature it did \
         not name"
    );
    assert!(
        in_graveyard(&engine, p0, monstrous_growth()).is_some(),
        "the sorcery resolved and went to its owner's graveyard"
    );
}
