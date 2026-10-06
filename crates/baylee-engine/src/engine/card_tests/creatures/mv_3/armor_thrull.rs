//! `cards/creatures/mv_3/armor_thrull.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Armor Thrull — {2}{B} 1/3: "{T}, Sacrifice this creature: Put a +1/+2
/// counter on target creature."
///
/// The counter is +1/+2 and not +1/+1, so the target's body after the
/// activation is the whole card: `(1, 1)` would mean nothing was placed and
/// `(2, 2)` that the toughness half was read as power. "Target creature" is
/// the word that needs a witness on both sides of the table — an Elf across it
/// is a legal target and an Elf that was not named must stay a printed 1/1 —
/// and the sacrifice, being a cost (CR 601.2h), is read after the target
/// question is answered and before the counter ever lands.
#[test]
fn armor_thrull_sacrifices_itself_for_a_plus_one_plus_two_counter_on_the_creature_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[armor_thrull()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");

    // {2}{B} off three Swamps; the Elves are named as the printing kept back,
    // so the board the counter lands on is the one that was dealt.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Swamps, and no creature tapped for mana"
    );
    cast_with_floating(&mut engine, p0, armor_thrull());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && on_battlefield(e, p0, armor_thrull()).is_some()
    });
    let thrull = on_battlefield(&engine, p0, armor_thrull()).expect("the Thrull resolved");
    assert_eq!(pt(&engine, thrull), (1, 3), "the body the card prints");
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "a printed 1/1 before anything is placed"
    );

    // CR 302.6: a creature that arrived this turn cannot pay a `{T}`, so
    // the turn goes round once before the line is pressed. Both halves are
    // needed — `walk_to_own_main` on its own returns where it stands,
    // because this already *is* p0's own main phase.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the turn came back round, and the summoning sickness with it"
    );
    activate(&mut engine, p0, armor_thrull(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that chooses");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    // CR 601.2c names the target before CR 601.2h pays, so the Thrull is still
    // standing while this question is open — the sacrifice is a cost, not a
    // rider on the effect.
    assert!(
        on_battlefield(&engine, p0, armor_thrull()).is_some(),
        "the cost is paid after the target, not before it"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the Elf was one of the options it enumerated");

    assert!(
        on_battlefield(&engine, p0, armor_thrull()).is_none(),
        "sacrificing itself is half of the price the card prints"
    );
    assert!(
        in_graveyard(&engine, p0, armor_thrull()).is_some(),
        "and a sacrificed creature goes to its owner's graveyard"
    );
    assert!(
        !stack_is_empty(&engine),
        "placing a counter is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, mine),
        (2, 3),
        "+1/+2 on the creature that was named — a (2, 2) would mean the \
         toughness half was read as power"
    );
    assert_eq!(
        counters_on(
            &engine,
            mine,
            CounterKind::Plus {
                power: 1,
                toughness: 2
            }
        ),
        1,
        "one counter, of the kind the card prints"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the Elf the ability did not name never moved"
    );
}
