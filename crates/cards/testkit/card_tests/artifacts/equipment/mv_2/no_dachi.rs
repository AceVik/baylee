//! `cards/artifacts/equipment/mv_2/no_dachi.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// No-Dachi — {2} Equipment: "Equipped creature gets +2/+0 and has first
/// strike. Equip {3}". Both printed statics are `Filter::AttachedToBySource`,
/// so the reading worth playing is the one that tells the creature the
/// Equipment *holds* from every other creature on the table: an unequipped
/// Elf beside the host stays a printed 1/1, and so does the Elf across it,
/// which the equip's own "target creature you control" must also decline to
/// offer. The five Forests pay the {2} and then the {3} out of the mana still
/// floating in the same main phase (CR 500.5), so the pool reads zero once the
/// host is armed and nothing about the offer is a label on a free ability.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn no_dachi_grants_two_power_and_first_strike_to_the_creature_it_holds() {
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
                forest(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[no_dachi()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "a printed 1/1 before the Equipment"
    );
    assert!(
        !keywords(&engine, host).contains(KeywordSet::FIRST_STRIKE),
        "nothing is equipped yet"
    );

    // Five Forests pay the {2} and leave the {3} the equip charges beside it
    // in the pool; the Elves are kept back so that "five" is the Forests and
    // no creature on this board was tapped for mana instead.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Forests, and no creature tapped for mana"
    );
    cast_with_floating(&mut engine, p0, no_dachi());
    pass_until(&mut engine, stack_is_empty);
    let equipment = on_battlefield(&engine, p0, no_dachi()).expect("the Equipment resolved");
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
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the {{2}} is spent and the {{3}} the equip will charge is still in the pool"
    );

    // The ability index comes out of the offer rather than being guessed: the
    // equip is the only *activated* ability the card prints, the two statics
    // behind it are never offered at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == equipment)
        .expect("Equip {3} is the only activated ability No-Dachi prints");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the three mana already floating pays the equip");

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
        (3, 1),
        "+2/+0 on the creature the Equipment is attached to"
    );
    assert!(
        keywords(&engine, host).contains(KeywordSet::FIRST_STRIKE),
        "and the printed first strike reaches it through the layers"
    );
    assert!(
        !keywords(&engine, equipment).contains(KeywordSet::FIRST_STRIKE),
        "the Equipment grants the keyword, it does not keep it"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody equipped is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the statics reach the equipped creature and never across the table"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the equip's {{3}} came out of the pool"
    );
}
