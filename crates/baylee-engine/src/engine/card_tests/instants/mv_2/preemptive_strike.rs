//! `cards/instants/mv_2/preemptive_strike.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Preemptive Strike — {1}{U} instant: "Counter target creature spell."
///
/// The scenario is the one the card exists for: an opponent casts a creature,
/// so the spell sits on the stack with no creature anywhere near the
/// battlefield. The counter is read in the *zone* the card lands in — a
/// countered spell goes to its owner's graveyard (CR 701.6a) and never enters,
/// so one resolution has to show both: the Elves in p1's graveyard and no Elf
/// under p1's control. The target question is the other half: the spell on the
/// menu is the object that was just cast, and the {1}{U} leaves the pool only
/// after that answer (CR 601.2c before CR 601.2h), which is where the payment
/// is checked.
#[test]
fn preemptive_strike_counters_a_creature_spell_that_is_still_on_the_stack() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[preemptive_strike()])
        .battlefield(1, &[forest()])
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);

    // p1's turn, because the creature spell this instant answers has to be
    // theirs: the seat holding the counter is not the active one.
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        1,
        "one Forest pays the Elves' {{G}}"
    );
    cast_with_floating(&mut engine, p1, llanowar_elves());

    // Back to p0 with the creature spell waiting and nothing resolved yet.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    let elves = on_stack(&engine, llanowar_elves())
        .expect("the Elves are a spell on the stack, not a permanent");
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "a spell that has not resolved is no creature on the battlefield"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Islands, one more blue than {{1}}{{U}} asks for"
    );
    cast_with_floating(&mut engine, p0, preemptive_strike());

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature spell\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        player, p0,
        "the seat casting the instant is the one aiming it"
    );
    assert!(
        options.contains(&elves),
        "the creature spell just cast is the spell on the menu: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("the spell the question offered is the one being countered");

    assert!(
        on_stack(&engine, preemptive_strike()).is_some(),
        "the instant is on the stack beside the spell it answered"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{1}}{{U}} is the last step of the cast (CR 601.2h), leaving one \
         of the three Islands' blue behind"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "a countered spell is put into its owner's graveyard (CR 701.6a)"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "and it never resolved, so the battlefield never sees it"
    );
    assert!(
        in_graveyard(&engine, p0, preemptive_strike()).is_some(),
        "the instant itself resolved and is in its owner's graveyard"
    );
}
