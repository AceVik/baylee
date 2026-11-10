//! `cards/artifacts/equipment/mv_3/vulshok_battlegear.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Vulshok Battlegear — {3} Equipment: "Equipped creature gets +3/+3" and
/// "Equip {3}". An Equipment is only itself when both printed lines happen in
/// one game, so six Forests pay for both at once: the {3} that brings the
/// artifact to the table and the {3} that attaches it, which is what tells a
/// real equip payment from a label on a free ability. The +3/+3 is read off the
/// creature the Equipment *holds* — a second Elf under the same seat and a
/// third across the table both stay printed 1/1s — and (4, 4) on a printed 1/1
/// is the only number that applies both the +3 and the word "equipped".
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn vulshok_battlegear_costs_three_to_equip_and_gives_three_to_the_creature_it_holds() {
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
        .hand(0, &[vulshok_battlegear()])
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

    // Six Forests, and both Elves named as the things kept back: six is exactly
    // the {3} to cast and the {3} to equip, so neither payment is read off a
    // creature that has its own reasons to be tapped.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six tapped Forests and two untapped Elves"
    );
    cast_with_floating(&mut engine, p0, vulshok_battlegear());
    pass_until(&mut engine, stack_is_empty);
    let gear = on_battlefield(&engine, p0, vulshok_battlegear()).expect("the Equipment resolved");
    assert!(
        engine
            .state()
            .object(gear)
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
        "the {{3}} is spent and the {{3}} the equip charges is still in the pool"
    );

    // Equip {3}, taken out of the offer rather than guessed at: the static that
    // grants the +3/+3 is never offered, so the one entry under this source is
    // the equip and there is nothing to count.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == gear)
        .expect("Equip {3} is the only activated ability the card prints");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the lands already tapped are the {3}");

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
        !options.contains(&gear),
        "the Equipment is an artifact and no creature: {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");

    // CR 601.2c picked the target and CR 601.2h pays afterwards, so the {3} is
    // still in the pool while the question stands.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the equip's cost is the last step of the activation"
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
            .object(gear)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    assert_eq!(
        pt(&engine, host),
        (4, 4),
        "+3/+3 on the creature the Equipment holds"
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
        "and the equip's {{3}} came out of the pool"
    );
}
