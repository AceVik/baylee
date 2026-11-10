//! `cards/creatures/mv_3/fire_imp.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fire Imp is `{2}{R}` for a 2/1 whose whole text is "When this creature
/// enters, it deals 2 damage to target creature" — a trigger, a target and a
/// number, none of which the card file can be asked about. The board carries
/// three creatures so that both halves of `Filter::CREATURE` are struck: the
/// offer has to name the Elf under the Imp's *own* control beside the two
/// across the table, because the card prints no "you control", and the two
/// damage has to fall on the single Elf that was named, because the card
/// prints "target creature" and not "each". The Sol Ring across the table is
/// the counter-half of the filter, and the Elf nobody named is what keeps the
/// lethal 1/1 from being read as a board sweep.
#[test]
fn fire_imp_deals_two_damage_to_the_creature_it_names_and_no_other() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), llanowar_elves()])
        .hand(0, &[fire_imp()])
        .battlefield(1, &[llanowar_elves(), llanowar_elves(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = all_on_battlefield(&engine, p1, llanowar_elves());
    assert_eq!(theirs.len(), 2, "two Elves across the table");
    let (victim, bystander) = (theirs[0], theirs[1]);
    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    assert_eq!(
        pt(&engine, victim),
        (1, 1),
        "a printed 1/1 for the Imp's two damage"
    );

    // Three Mountains pay the {2}{R}; the Elf is named as the printing to keep
    // back, so what pays for the Imp is the lands and not a creature this test
    // reads again afterwards.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Mountains, and the Elf kept back"
    );
    cast_with_floating(&mut engine, p0, fire_imp());

    // The spell resolves, the Imp enters, and its enters-trigger asks for the
    // one target the card prints.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(
        player, p0,
        "the controller of the trigger chooses its target"
    );
    assert!(
        options.contains(&mine) && options.contains(&victim) && options.contains(&bystander),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&rock),
        "a Sol Ring is an artifact and no creature: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("the Elf across the table was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "two damage to a printed 1/1 is lethal (CR 704.5g)"
    );
    assert!(
        all_on_battlefield(&engine, p1, llanowar_elves()).contains(&bystander),
        "the Elf nobody named is still the 1/1 it was printed as"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and so is the creature under the Imp's own control, which the card \
         was free to name and this test did not"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the Sol Ring was never a target and never moved"
    );
    let imp = on_battlefield(&engine, p0, fire_imp()).expect("the Imp resolved and stayed");
    assert_eq!(
        pt(&engine, imp),
        (2, 1),
        "\"it deals 2 damage\": the ability damages and the body is untouched"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{2}}{{R}} it cost came out of the pool the Mountains filled"
    );
}
