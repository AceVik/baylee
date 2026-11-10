//! `cards/creatures/mv_1/cabal_trainee.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Cabal Trainee prints `{B}` for a 1/1 and one line: "Sacrifice this
/// creature: Target creature gets -2/-0 until end of turn."
///
/// The ability's whole price is the creature itself, which is what makes the
/// order of the two questions the point: targets are announced first
/// (CR 601.2c) and costs are paid last (CR 601.2h), so the Trainee is still
/// standing — and still a legal answer of its own — while the target menu is
/// open, and gone the instant it is answered. The target is a Rootbreaker
/// Wurm across the table, with an Elf beside the Trainee as the control: the
/// filter is `Filter::CREATURE` and not "you control", so the offer has to
/// name both sides of the table, and the result is read as a *change* —
/// power down exactly two, toughness untouched — so -2/-0 is told from
/// -2/-2 without hard-coding a body the card file would have to agree with.
#[test]
fn cabal_trainee_sacrifices_itself_for_two_power_off_any_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), llanowar_elves()])
        .hand(0, &[cabal_trainee()])
        .battlefield(1, &[rootbreaker_wurm()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, cabal_trainee());
    pass_until(&mut engine, stack_is_empty);
    let _trainee = on_battlefield(&engine, p0, cabal_trainee()).expect("the Trainee resolved");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, rootbreaker_wurm()).expect("their Wurm is out");
    let (power, toughness) = pt(&engine, theirs);

    // Ability 0 is the only thing the card prints, and its cost is the
    // creature: it is offered on an empty pool in its own main phase.
    activate(&mut engine, p0, cabal_trainee(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the ability targets a creature, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&mine),
        "\"target creature\" offers the Trainee's own side of the table: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "and reaches across it, which is the word \"you control\" missing: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p0, cabal_trainee()).is_some(),
        "CR 601.2c before CR 601.2h: the sacrifice is still unpaid while the \
         target is being chosen"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Wurm was one of the options");

    assert!(
        on_battlefield(&engine, p0, cabal_trainee()).is_none(),
        "naming the target is what paid the cost, so the Trainee is gone"
    );
    assert!(
        in_graveyard(&engine, p0, cabal_trainee()).is_some(),
        "and a sacrificed creature is in its owner's graveyard"
    );
    assert_eq!(
        on_battlefield(&engine, p0, cabal_trainee()),
        None,
        "nowhere else for it to have gone"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, theirs),
        (power - 2, toughness),
        "-2/-0 on the creature it was aimed at: two off the power and \
         nothing at all off the toughness"
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "and nothing at all on the creature it was not aimed at"
    );
}
