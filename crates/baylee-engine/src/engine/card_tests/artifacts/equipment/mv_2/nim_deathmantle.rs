//! `cards/artifacts/equipment/mv_2/nim_deathmantle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Nim Deathmantle prints four clauses and three of them are written: the
/// equipped creature gets +2/+2, is black, and is a Zombie, and the Equip is
/// `{4}`. The statics are `Filter::AttachedToBySource`, so the only reading
/// worth playing is the one that tells the creature the Equipment *holds*
/// from every other creature in the game — which is why the Elves across the
/// table are read, and why the host is a `(1, 1)` before the equip and a
/// `(3, 3)` after it. The six Forests are the other half: they pay the `{2}`
/// and leave exactly the `{4}` the equip charges, so the artifact arrives by
/// being cast and the cost that lands the keywords on the host is a real
/// payment out of the pool rather than a label.
#[test]
fn nim_deathmantle_equips_for_four_and_clamps_only_the_creature_it_holds() {
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
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[nim_deathmantle()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let host = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, host), (1, 1), "nothing is equipped yet");

    // The artifact has to arrive, not merely be believed in: six tapped
    // Forests pay the {2} and leave exactly the {4} the equip asks for. The
    // Elf is kept back so that "exactly" is the six Forests and not seven
    // sources — it is the creature the Equipment is about to hold.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, nim_deathmantle());
    pass_until(&mut engine, stack_is_empty);
    let mantle = on_battlefield(&engine, p0, nim_deathmantle()).expect("the Deathmantle resolved");
    assert!(
        engine
            .state()
            .object(mantle)
            .is_some_and(|o| o.attached_to.is_none()),
        "an Equipment enters holding nobody"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the {{2}} is spent and the {{4}} the equip will charge is still in the pool"
    );
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "an Equipment attached to nothing modifies nothing"
    );

    // Equip {4}: ability 3 on the card, behind the three statics that do the
    // granting. That it is offered at all is the pool reading above.
    activate(&mut engine, p0, nim_deathmantle(), 3);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        options,
        vec![host],
        "target creature *you* control — the Elves across the table are not offered"
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
            .object(mantle)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    assert_eq!(
        pt(&engine, host),
        (3, 3),
        "+2/+2 for the creature the Equipment holds"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and nothing at all for a creature it does not hold"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the equip's {{4}} came out of the pool"
    );
}
