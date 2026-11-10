//! `cards/artifacts/equipment/mv_1/slagwurm_armor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Slagwurm Armor — {1} artifact, Equipment. "Equipped creature gets
/// +0/+6" and "Equip {3}". The static is a `Filter::AttachedToBySource`
/// one, so the reading worth playing is the one that tells the creature
/// the Armor *holds* from every other creature on the table: two Elves
/// stand under the same seat and one across it, and the printed 1/1 of
/// each is what makes the equipped Elf's 1/7 a filter and not a board
/// buff. The {3} is a real payment rather than a label — six mana come off
/// the four Forests and the two Elves, and both the pool read while the
/// target is still unanswered (CR 601.2h) and the one after the equip
/// resolves say so.
#[test]
#[allow(clippy::too_many_lines)] // one equip, and every clause of the card read off it
fn slagwurm_armor_grants_six_toughness_to_the_creature_it_holds_and_no_other() {
    fn slagwurm_armor() -> CardIndex {
        card_index("20b60c93-124b-42c8-93fb-63bbd1888658")
    }

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
        .hand(0, &[slagwurm_armor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Armor");

    // Four Forests and two Elves are six mana, and the Elves are tapped for
    // it the same way the lands are: what is left in the pool afterwards is
    // the six less the {1} the Armor costs.
    cast_from_hand(&mut engine, p0, slagwurm_armor());
    pass_until(&mut engine, stack_is_empty);
    let armor = on_battlefield(&engine, p0, slagwurm_armor()).expect("the Armor resolved");
    assert!(
        engine
            .state()
            .object(armor)
            .is_some_and(|o| o.attached_to.is_none()),
        "an Equipment enters unattached and stays on the battlefield"
    );
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "an Equipment attached to nothing modifies nothing"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "six mana off the board less the {{1}} the Armor cost"
    );

    // Mana before the claim: the offer is read off the pool and not off the
    // untapped lands, and the index is taken out of the offer rather than
    // guessed — the equip is the only *activated* ability the card prints.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == armor)
        .expect("Equip {3} is the only activated ability the Armor prints");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the five mana already floating are the {3}");

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
        !options.contains(&armor),
        "the Equipment is an artifact and no creature: {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "CR 601.2h pays last: the target is answered first, so nothing is \
         spent while the question stands"
    );
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
            .object(armor)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the equip's {{3}} came out of the pool"
    );
    assert_eq!(
        pt(&engine, host),
        (1, 7),
        "+0/+6 on the creature the Armor is attached to"
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
