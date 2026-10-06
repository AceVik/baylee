//! `cards/artifacts/equipment/mv_1/shuko.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Shuko is a {1} Equipment printing two lines: "equipped creature gets
/// +1/+0" and "Equip {0}". The free equip is what makes the first line
/// readable — the pump has to land on the creature the Equipment holds and
/// on no other, so the board carries two Elves under the same seat (one stays
/// bare), a Sol Ring which is an artifact and no creature, and an Elf across
/// the table that "target creature you control" must decline. Reading the
/// card file cannot tell "attached to" from "creatures you control": a static
/// that had lost `Filter::AttachedToBySource` would satisfy every number here.
#[test]
fn shuko_equips_for_free_and_pumps_only_the_creature_it_holds() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                quiet_artifact(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        // An Elf across the table, so "you control" is read and not assumed.
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[shuko()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Shuko");

    // The artifact arrives the way the artifact arrives: {1} off an open
    // board. Every source is tapped for it, which is why the offer below is
    // read afterwards and nothing is tapped again by hand.
    cast_from_hand(&mut engine, p0, shuko());
    pass_until(&mut engine, stack_is_empty);
    let equipment = on_battlefield(&engine, p0, shuko()).expect("the Shuko resolved");
    assert!(
        engine
            .state()
            .object(equipment)
            .is_some_and(|o| o.attached_to.is_none()),
        "an Equipment enters holding nobody"
    );
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "an Equipment attached to nothing modifies nothing"
    );

    // The whole price of the equip is the {0} it prints, so whatever is
    // floating before the activation must still be floating after it.
    let floating = engine.state().players[0].mana_pool.total();

    // Ability 0 is Equip {0}; ability 1 is the static that grants.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: equipment,
                ability_index: 0,
            },
        )
        .expect("equip is offered");
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
        !options.contains(&equipment),
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
            .object(equipment)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    assert_eq!(
        pt(&engine, host),
        (2, 1),
        "equipped creature gets +1/+0 — power up, toughness untouched"
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
        floating,
        "Equip {{0}} charges nothing: the pool is exactly where the cast left it"
    );
}
