//! `cards/creatures/mv_2/wall_of_roots.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wall of Roots ({1}{G}, 0/5): "Put a -0/-1 counter on this creature: Add
/// {G}. Activate only once each turn."
///
/// **The limit is spent when the ability is activated, and the turn is the
/// unit.** CR 602.2 makes activating an ability putting it on the stack and
/// paying its costs, so the count moves at the payment and not at the
/// resolution — which for a mana ability are one moment anyway (CR 605.3b),
/// and for the next card that prints this clause will not be.
///
/// **Each turn, not each of *your* turns.** The sentence says neither "only
/// during your turn" nor "only once each of your turns", so a Wall used in
/// its controller's main phase is available again while the opponent is
/// taking theirs — which is the half a tally cleared at the wrong moment
/// would get wrong in the player's favour, and the half worth two extra
/// passes to assert.
///
/// The cost is also why the Wall is not a tapper: nothing in it says {T}, so
/// summoning sickness never touched it and neither does tapping. What stops
/// the second activation is the printed sentence and nothing else.
#[test]
fn wall_of_roots_may_only_be_asked_once_a_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(3307, forest())
        .battlefield(0, &[wall_of_roots()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let wall = on_battlefield(&engine, p0, wall_of_roots()).expect("the Wall is on the table");
    assert_eq!(pt(&engine, wall), (0, 5), "a 0/5 to start with");

    let offered = |engine: &Engine<RegistryLookup>| -> bool {
        matches!(
            engine.pending(),
            Pending::Priority { legal, .. } if legal.abilities.contains(&(wall, 0))
        )
    };
    assert!(offered(&engine), "the first activation is offered");

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: wall,
                ability_index: 0,
            },
        )
        .expect("a permanent can always take a counter, so the cost is payable");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "one green, and no stack: a mana ability resolves as it is activated"
    );
    assert_eq!(
        counters_on(
            &engine,
            wall,
            CounterKind::Minus {
                power: 0,
                toughness: 1
            }
        ),
        1,
        "the counter it paid with"
    );
    assert_eq!(
        pt(&engine, wall),
        (0, 4),
        "-0/-1 takes a toughness, so the Wall is a 0/4"
    );
    assert!(
        !offered(&engine),
        "and the printed sentence takes the ability off the offer for the \
         rest of the turn — an ability nobody is offered is the whole of the \
         refusal, the way every other activation restriction works here"
    );

    // The opponent's turn is a different turn, and the Wall's controller has
    // priority during it.
    reach_their_main_phase(&mut engine, p1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    assert!(
        offered(&engine),
        "\"only once each turn\" is not \"only once each of your turns\""
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: wall,
                ability_index: 0,
            },
        )
        .expect("the second turn's activation");
    assert_eq!(
        pt(&engine, wall),
        (0, 3),
        "two counters now, and still only one a turn"
    );
    assert!(!offered(&engine), "spent again, in the same one-a-turn way");
}

/// A -0/-1 counter and a +1/+1 counter on the same creature, which is two
/// rules at once.
///
/// CR 122.1a gives every +X/+Y counter its own arithmetic, so layer 7c
/// (CR 613.4c) has to sum power and toughness **apart**: one shared delta is
/// right only while every counter is symmetric, which was true of the two
/// counters this pool used to print and is true of nothing else. A Wall of
/// Roots wearing both is a 1/5 — the plus moved the power the minus did not
/// touch.
///
/// And they do **not** cancel. CR 704.5q annihilates a +1/+1 against a
/// -1/-1 and no other pair, so both counters are still there afterwards;
/// a state-based action that read "any plus against any minus" would leave
/// the Wall a bare 0/5 and this test would say so.
///
/// Abandoned Air Temple is the +1/+1, because its ability puts one on *each*
/// creature its controller has rather than on a target — no question to
/// answer, and nothing between the activation and the counter.
#[test]
fn a_minus_zero_one_counter_takes_a_toughness_and_leaves_the_power() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(3308, forest())
        .battlefield(
            0,
            &[
                wall_of_roots(),
                abandoned_air_temple(),
                plains(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let wall = on_battlefield(&engine, p0, wall_of_roots()).expect("the Wall is on the table");
    let temple =
        on_battlefield(&engine, p0, abandoned_air_temple()).expect("the Temple is on the table");
    assert_eq!(pt(&engine, wall), (0, 5), "a 0/5 to start with");

    // {3}{W} out of the Plains and three Forests, then the Temple's own tap.
    tap_all_mana_but(&mut engine, p0, Some(abandoned_air_temple()));
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: temple,
                ability_index: 1,
            },
        )
        .expect("{3}{W} and a tap buys a counter for every creature");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, wall),
        (1, 6),
        "the +1/+1 moved both numbers, which is what a symmetric counter does"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: wall,
                ability_index: 0,
            },
        )
        .expect("the Wall pays a -0/-1 for its green");
    assert_eq!(
        pt(&engine, wall),
        (1, 5),
        "and the -0/-1 took a toughness and left the power alone"
    );
    assert_eq!(
        counters_on(&engine, wall, CounterKind::P1P1),
        1,
        "the +1/+1 is still there: CR 704.5q cancels it against a -1/-1 and \
         against nothing else"
    );
    assert_eq!(
        counters_on(
            &engine,
            wall,
            CounterKind::Minus {
                power: 0,
                toughness: 1
            }
        ),
        1,
        "and so is the -0/-1"
    );
}
