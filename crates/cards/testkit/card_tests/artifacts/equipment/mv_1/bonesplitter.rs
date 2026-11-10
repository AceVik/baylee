//! `cards/artifacts/equipment/mv_1/bonesplitter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Bonesplitter — {1} Equipment: "Equipped creature gets +2/+0" and "Equip
/// {1}".
///
/// `(3, 1)` is the only answer that reads both printed numbers: a host still
/// at `(1, 1)` means the static never applied, and `(3, 3)` that a toughness
/// half that is not on the card was invented. The unequipped Elf beside the
/// host and the Elf across the table are the two halves of the filter —
/// "equipped creature" is neither "creatures you control" nor "creatures",
/// and a host-only reading could not tell those apart. Exactly the two
/// Forests are tapped, the Elves kept back, so the mana the pool is missing
/// afterwards is the printed `{1}` actually paid rather than a source that
/// happened to move.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn bonesplitter_gives_two_power_to_the_creature_it_holds_and_to_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(31, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                bonesplitter(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    let splitter = on_battlefield(&engine, p0, bonesplitter()).expect("the Equipment is out");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the equip");
    assert!(
        engine
            .state()
            .object(splitter)
            .is_some_and(|o| o.attached_to.is_none()),
        "an Equipment enters holding nobody"
    );

    // Two Forests and only those: both Elves are the creatures this test
    // reads back afterwards, and `tap_all_mana` would have drunk their
    // own `{T}: Add {G}` as well.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Forests tapped and no Elf: the Elves' mana ability is a printed \
         one and `tap_all_mana_but` is what keeps it out of this"
    );

    // Equip {1} is the only activated ability the Equipment prints, so the
    // index is taken out of the offer rather than guessed.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == splitter)
        .expect("Equip {1} is offered once the mana is floating");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the two Forests in the pool are the {1}");

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures you control may wear it: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"target creature *you* control\" declines the Elf across the table: {options:?}"
    );
    assert!(
        !options.contains(&splitter),
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
            .object(splitter)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    assert_eq!(
        pt(&engine, host),
        (3, 1),
        "+2/+0 on the creature the Equipment is attached to"
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
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the equip's {{1}} came out of the pool the two Forests filled"
    );
}
