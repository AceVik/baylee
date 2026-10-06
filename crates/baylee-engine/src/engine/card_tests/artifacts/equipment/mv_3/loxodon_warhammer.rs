//! `cards/artifacts/equipment/mv_3/loxodon_warhammer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Loxodon Warhammer — {3} Artifact — Equipment: "Equipped creature gets
/// +3/+0 and has trample and lifelink. Equip {3}."
///
/// The grant is a `Filter::AttachedToBySource` static, so the reading worth
/// playing is the one that tells the creature the Hammer *holds* from every
/// other creature on the table: an unequipped Elf beside the host and an Elf
/// across the table have to stay printed 1/1s with neither keyword while the
/// host reads 4/1 and carries both. `(4, 1)` is the only body that reads the
/// printed +3/+0 — a `(4, 4)` would be a toughness pump the card never had —
/// and the equip is a real {3} out of a pool the lands and Elves actually
/// paid into, not a label on a free ability.
#[test]
#[allow(clippy::too_many_lines)] // one play, and every clause of the card read off it
fn loxodon_warhammer_pumps_and_arms_the_creature_it_holds_and_no_other() {
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
                // Seated rather than cast: what this test reads is the equip
                // and the grant, and paying {3} for the Hammer first would
                // take that {3} out of the pool the equip is measured
                // against. An Equipment prints no enter modifier, so a
                // placement is the same permanent a cast would have left.
                loxodon_warhammer(),
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
    let hammer = on_battlefield(&engine, p0, loxodon_warhammer()).expect("the Hammer is seated");
    assert!(
        engine
            .state()
            .object(hammer)
            .is_some_and(|o| o.attached_to.is_none()),
        "an Equipment enters holding nobody"
    );
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Hammer");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::TRAMPLE),
        "an Equipment attached to nothing grants nothing"
    );

    // Mana before the claim (the offer is read off the pool, not off the
    // board), and the index taken out of the offer rather than guessed:
    // Equip {3} is the only *activated* ability the card prints.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "four Forests and the two Elves, which is every mana source on the board"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let offered: Vec<(ObjectId, u32)> = legal
        .abilities
        .iter()
        .copied()
        .filter(|(src, _)| *src == hammer)
        .collect();
    assert_eq!(
        offered.len(),
        1,
        "the two statics are never activated, so the equip is the whole offer: {offered:?}"
    );
    activate(&mut engine, p0, loxodon_warhammer(), offered[0].1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures you control may take the Hammer: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"target creature *you* control\" declines the Elf across the table: {options:?}"
    );
    assert!(
        !options.contains(&hammer),
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
            .object(hammer)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    assert_eq!(
        pt(&engine, host),
        (4, 1),
        "+3/+0 on the creature the Hammer holds — a (4, 4) would mean a \
         toughness the card does not print"
    );
    let granted = keywords(&engine, host);
    assert!(
        granted.contains(KeywordSet::TRAMPLE),
        "equipped creature has trample"
    );
    assert!(
        granted.contains(KeywordSet::LIFELINK),
        "and lifelink, from the same granted set"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody equipped is still the 1/1 it was printed as"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::TRAMPLE),
        "the static reaches the equipped creature and no other"
    );
    assert_eq!(pt(&engine, theirs), (1, 1), "and never across the table");
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::LIFELINK),
        "\"equipped creature\" is not \"creatures you control\", let alone \
         anyone else's"
    );
    assert!(
        !keywords(&engine, hammer).contains(KeywordSet::TRAMPLE),
        "the Equipment grants the keywords, it does not keep them"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the equip's {{3}} came out of the pool"
    );
}
