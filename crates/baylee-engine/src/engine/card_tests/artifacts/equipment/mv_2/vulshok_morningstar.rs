//! `cards/artifacts/equipment/mv_2/vulshok_morningstar.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Vulshok Morningstar prints exactly two lines: "Equipped creature gets
/// +2/+2" and "Equip {2}". The static is `Filter::AttachedToBySource`, so the
/// only reading worth playing is one that tells the creature the Equipment
/// *holds* from every other creature on the table — the unequipped Elf beside
/// the host and the Elf across it must both stay 1/1s while the host becomes a
/// 3/3. The equip is a real {2} out of a pool the four Forests actually paid
/// into: the mana is still floating while the target question is open
/// (CR 601.2c before CR 601.2h), and the pool is empty once the host has been
/// answered and the equip has resolved.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn vulshok_morningstar_arms_the_creature_it_holds_and_no_other() {
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
        .hand(0, &[vulshok_morningstar()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "a printed 1/1 before the Equipment"
    );

    // The four Forests pay both halves — the cast's {2} and the equip's {2} —
    // inside this one main phase, so CR 500.5 never empties the pool between
    // them. Both Elves are named as kept back: they tap for mana of their own,
    // and a pool of six would make every number below a claim about mana
    // nothing on the board accounted for.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Forests, and neither Elf tapped"
    );

    cast_with_floating(&mut engine, p0, vulshok_morningstar());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, vulshok_morningstar()).is_some()
    });
    let star =
        on_battlefield(&engine, p0, vulshok_morningstar()).expect("the Morningstar resolved");
    assert!(
        engine
            .state()
            .object(star)
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
        2,
        "the cast's {{2}} is spent and the equip's {{2}} is not"
    );

    // Equip {2}. The index comes out of the offer rather than out of the card
    // file: the equip is the only *activated* ability the card prints, and the
    // offer is only non-empty because the pool already holds the two mana.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == star)
        .expect("Equip {2} is the only activated ability the Morningstar prints");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the two mana already floating pay for it");

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
        !options.contains(&star),
        "the Equipment is an artifact and no creature: {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "CR 601.2h: the cost follows the target, so the mana is still floating"
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
            .object(star)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    assert_eq!(
        pt(&engine, host),
        (3, 3),
        "+2/+2 on the creature the Morningstar holds"
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
        0,
        "and the equip's {{2}} came out of the pool"
    );
}
