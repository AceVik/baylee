//! `cards/artifacts/equipment/mv_1/basilisk_collar.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Basilisk Collar prints two sentences: "Equipped creature has deathtouch
/// and lifelink", and "Equip {2}". The grant is a
/// `Filter::AttachedToBySource` static, so the only reading worth playing is
/// the one that tells the creature the Equipment *holds* from every other
/// creature on the table: an unequipped Elf beside the host and an Elf across
/// the table both stay keywordless, and the equip itself is a real {2} out of
/// a pool the Forests actually paid into — nothing is asserted about the
/// offer until the mana is already floating.
#[test]
#[allow(clippy::too_many_lines)] // one play, and every clause of the card read off it
fn basilisk_collar_grants_deathtouch_and_lifelink_to_the_creature_it_holds() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, forest())
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
        .hand(0, &[basilisk_collar()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::DEATHTOUCH),
        "nothing is equipped yet"
    );

    // {1} off one Forest, and the rest of the four stay in the pool for the
    // {2} the equip charges — the same phase, so CR 500.5 does not empty it.
    cast_from_hand(&mut engine, p0, basilisk_collar());
    pass_until(&mut engine, stack_is_empty);
    let collar = on_battlefield(&engine, p0, basilisk_collar()).expect("the Collar resolved");
    assert!(
        engine
            .state()
            .object(collar)
            .is_some_and(|o| o.attached_to.is_none()),
        "an Equipment enters holding nobody"
    );
    assert!(
        !keywords(&engine, host).contains(KeywordSet::DEATHTOUCH),
        "an Equipment attached to nothing grants nothing"
    );

    // Mana before the claim (the offer is read off the pool, not off the
    // board), and the index taken out of the offer rather than guessed: the
    // equip is the only *activated* ability the card prints.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == collar)
        .expect("Equip {2} is the only activated ability the Collar prints");
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
        !options.contains(&collar),
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
            .object(collar)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    let granted = keywords(&engine, host);
    assert!(
        granted.contains(KeywordSet::DEATHTOUCH),
        "equipped creature has deathtouch"
    );
    assert!(granted.contains(KeywordSet::LIFELINK), "and lifelink");
    assert!(
        !keywords(&engine, collar).contains(KeywordSet::DEATHTOUCH),
        "the Equipment grants the keywords, it does not keep them"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::LIFELINK),
        "the static reaches the equipped creature and no other"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::DEATHTOUCH),
        "nor across the table"
    );
}
