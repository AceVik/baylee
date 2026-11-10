//! `cards/artifacts/equipment/mv_1/neurok_hoversail.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Neurok Hoversail is two printed sentences and the test plays both: "{1}"
/// for the Equipment and "Equip {2}" to land "Equipped creature has flying"
/// on a creature. The static is `Filter::And(&[Filter::CREATURE,
/// Filter::AttachedToBySource])`, so the reading worth playing is the one
/// that tells the creature the artifact *holds* from every other creature on
/// the table — which is why an unequipped Elf beside the host and an Elf
/// across the table both stay grounded. The four Forests pay the {1} and
/// leave exactly the {2} the equip charges, so the keyword arrives off a real
/// payment out of the pool and not off a label, and the Elves are kept
/// untapped so the creatures the equip offers are the ones it was aimed at.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn neurok_hoversail_grants_flying_to_the_creature_it_equips_and_no_other() {
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
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[neurok_hoversail()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(
        elves.len(),
        2,
        "two Elves, one of which stays on the ground"
    );
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::FLYING),
        "nothing is equipped yet"
    );

    // The Elves are named as the thing kept back: they are the creatures the
    // equip is about to choose between, and a source tapped for mana is a
    // source whose status has already changed for a reason of its own.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Forests tapped, and the two Elves still standing"
    );
    cast_with_floating(&mut engine, p0, neurok_hoversail());
    pass_until(&mut engine, stack_is_empty);
    let sail = on_battlefield(&engine, p0, neurok_hoversail()).expect("the Hoversail resolved");
    assert!(
        engine
            .state()
            .object(sail)
            .is_some_and(|o| o.attached_to.is_none()),
        "an Equipment enters holding nobody"
    );
    assert!(
        !keywords(&engine, host).contains(KeywordSet::FLYING),
        "an Equipment attached to nothing grants nothing"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the {{1}} is spent and the {{2}} the equip will charge is still floating"
    );

    // The equip is the only *activated* ability the card prints, taken out of
    // the offer rather than guessed: the static behind it is never offered.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == sail)
        .expect("Equip {2} is the only activated ability the Hoversail prints");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the lands already tapped are the {2}");

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
        !options.contains(&sail),
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
            .object(sail)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    assert!(
        keywords(&engine, host).contains(KeywordSet::FLYING),
        "equipped creature has flying"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::FLYING),
        "the static reaches the equipped creature and no other"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "nor across the table"
    );
    assert!(
        !keywords(&engine, sail).contains(KeywordSet::FLYING),
        "the Equipment grants the keyword, it does not keep it"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the equip's {{2}} came out of the pool"
    );
}
