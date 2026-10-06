//! `cards/artifacts/equipment/mv_1/leonin_scimitar.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Leonin Scimitar prints two lines: "Equipped creature gets +1/+1" and
/// "Equip {1}". Both are only worth anything together — the second must
/// bind the first to exactly the creature that the equip question named —,
/// so next to the bearer stands a second Elf under the same control and a
/// third across the table: only the first may change, and `(2, 2)` versus
/// two times `(1, 1)` simultaneously rules out "creatures you control" and
/// "the whole table". Payment is made from three Plains, which leave both
/// Elves standing, so that the bearer was not itself tapped for its own
/// sword.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn leonin_scimitar_arms_only_the_creature_it_is_attached_to() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[leonin_scimitar()])
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
        "a printed 1/1 before the Scimitar"
    );

    // The {1} of the cast and the {1} of the equip come out of the same open
    // pool — a pool survives until the step ends (CR 500.5) and this whole
    // scenario lives in that one main phase. The Elves are named as the
    // printing to keep back: they are the creatures the equip is about, and
    // a host tapped for its own mana is a host that reads wrong afterwards.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Plains, and neither Elf tapped for it"
    );
    cast_with_floating(&mut engine, p0, leonin_scimitar());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && matches!(e.pending(), Pending::Priority { .. })
    });
    let scimitar = on_battlefield(&engine, p0, leonin_scimitar()).expect("the Scimitar resolved");
    assert!(
        engine
            .state()
            .object(scimitar)
            .is_some_and(|o| o.attached_to.is_none()),
        "an Equipment enters holding nobody"
    );
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "an Equipment attached to nothing modifies nothing"
    );

    // The equip is the only *activated* ability the card prints — a static
    // never reaches `legal.abilities` — so naming the source is enough, and
    // the index is taken out of the offer rather than guessed. Read with the
    // mana already floating, because `can_afford` reads the pool.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == scimitar)
        .expect("Equip {1} is the only activated ability the Scimitar prints");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the mana already floating pays for the equip");

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
        !options.contains(&scimitar),
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
            .object(scimitar)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    assert_eq!(
        pt(&engine, host),
        (2, 2),
        "+1/+1 on the creature the Scimitar is attached to"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody armed is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the static reaches the equipped creature and never across the table"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the two {{1}} costs came out of the three Plains"
    );
}
