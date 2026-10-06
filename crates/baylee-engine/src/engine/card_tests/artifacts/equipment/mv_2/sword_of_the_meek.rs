//! `cards/artifacts/equipment/mv_2/sword_of_the_meek.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sword of the Meek is `Coverage::Partial`: the printed static ("equipped
/// creature gets +1/+2") and the equip {2} are written, while the graveyard
/// return-and-attach trigger for a 1/1 entering is not. Two printed words
/// hold the written half up, and each needs a different bystander. "Equipped
/// creature" is not "creatures you control", so one unequipped Elf beside the
/// host is a live 1/1 at the end — and "you" is not "the table", so the Elf
/// across it must stay a printed 1/1 too. +1/+2 on a printed 1/1 reads
/// `(2, 3)`: a `(2, 2)` would mean the power was read twice and a `(1, 3)`
/// that the +1 was never applied at all.
#[test]
fn sword_of_the_meek_arms_the_creature_it_targets_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[sword_of_the_meek()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Sword");

    // {2} off two of the four Forests; the other two pay the equip.
    cast_from_hand(&mut engine, p0, sword_of_the_meek());
    pass_until(&mut engine, stack_is_empty);
    let sword = on_battlefield(&engine, p0, sword_of_the_meek()).expect("the Sword resolved");
    assert!(
        engine
            .state()
            .object(sword)
            .is_some_and(|o| o.attached_to.is_none()),
        "with nothing chosen yet it enters holding nobody"
    );
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "an Equipment attached to nothing modifies nothing"
    );

    // Ability 1 is Equip {2}; ability 0 is the static that grants.
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, sword_of_the_meek(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures you control may be armed: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"target creature *you* control\" declines the Elf across the table: {options:?}"
    );
    assert!(
        !options.contains(&sword),
        "the Equipment is an artifact and no creature: {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(sword)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    assert_eq!(
        pt(&engine, host),
        (2, 3),
        "+1/+2 on the creature the Sword is attached to"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody equipped is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the static reaches the equipped creature and never across the table"
    );
}
