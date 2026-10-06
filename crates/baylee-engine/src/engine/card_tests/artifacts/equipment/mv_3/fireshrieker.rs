//! `cards/artifacts/equipment/mv_3/fireshrieker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fireshrieker prints two sentences: "Equipped creature has double strike"
/// and "Equip {2}". The grant is a `Filter::AttachedToBySource` static, so
/// the only reading worth playing is the one that tells the creature the
/// Equipment *holds* from every other creature in the game — an unequipped
/// Elf beside the host and an Elf across the table both stay keywordless.
/// The six Forests are the other half: they pay the {3} and leave exactly
/// the {2} the equip charges, so the keyword lands on the host off a real
/// payment out of the pool and not off a label on a free ability.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn fireshrieker_arms_only_the_creature_it_holds_with_double_strike() {
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
                forest(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[fireshrieker()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::DOUBLE_STRIKE),
        "nothing is equipped yet"
    );

    // {3} off three of the six Forests, with the Elves named as the printing
    // kept back: `tap_all_mana` would have spent their own {T}, taking both
    // creatures out of the board this test goes on to read.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Forests, and neither Elf of mine paid in"
    );
    cast_with_floating(&mut engine, p0, fireshrieker());
    pass_until(&mut engine, stack_is_empty);
    let shrieker = on_battlefield(&engine, p0, fireshrieker()).expect("the Fireshrieker resolved");
    assert!(
        engine
            .state()
            .object(shrieker)
            .is_some_and(|o| o.attached_to.is_none()),
        "an Equipment enters holding nobody"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the {{3}} is spent and the {{2}} the equip will charge is still in the pool"
    );
    assert!(
        !keywords(&engine, host).contains(KeywordSet::DOUBLE_STRIKE),
        "an Equipment attached to nothing grants nothing"
    );

    // Mana before the claim, and the index taken out of the offer rather
    // than guessed: Equip {2} is the only *activated* ability the card
    // prints, and the static behind it is never offered at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == shrieker)
        .expect("Equip {2} is the only activated ability the Fireshrieker prints");
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
        !options.contains(&shrieker),
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
            .object(shrieker)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    assert!(
        keywords(&engine, host).contains(KeywordSet::DOUBLE_STRIKE),
        "equipped creature has double strike"
    );
    assert!(
        !keywords(&engine, shrieker).contains(KeywordSet::DOUBLE_STRIKE),
        "the Equipment grants the keyword, it does not keep it"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::DOUBLE_STRIKE),
        "the static reaches the equipped creature and no other"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::DOUBLE_STRIKE),
        "nor across the table"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and the equip's {{2}} came out of the pool"
    );
}
