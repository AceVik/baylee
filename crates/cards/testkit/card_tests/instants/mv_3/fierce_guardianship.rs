//! `cards/instants/mv_3/fierce_guardianship.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "d09c9cba-fdd2-479b-ad5d-d05181c3e3f9"

/// Fierce Guardianship prints "{2}{U} — Instant: If you control a commander, you
/// may cast this spell without paying its mana cost" and "Counter target
/// noncreature spell".
///
/// The free half is played on a seat that controls its commander and **no
/// untapped land**. The commander has to be *cast*: CR 903.6 starts it in the
/// command zone, and a second copy of the card seated on the battlefield is
/// not a commander at all. So seat 0 spends every land it has on it, and on
/// seat 1's turn the pool is empty and the alternative cost is the only way
/// the card can be offered; a board without a commander refuses it until the
/// printed {2}{U} is really floating — which is what separates the
/// printed condition from a card that is simply free. Seat 1 casts a creature
/// and then, still holding priority (CR 117.3c), an instant on top of it — an
/// artifact could not follow, because CR 301.1 wants an empty stack — so the
/// target menu reads the filter: the Brainstorm is offered, the Elves is not,
/// and the Elves is the spell that resolves behind the counter.
#[test]
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
fn fierce_guardianship_casts_free_under_a_commander_and_counters_only_the_noncreature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .commander(0, &[katara_the_fearless()])
        .battlefield(0, &[forest(), plains(), island()])
        .hand(0, &[fierce_guardianship()])
        .battlefield(1, &[forest(), island(), island()])
        .hand(1, &[llanowar_elves(), brainstorm()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The commander comes down for its printed {G}{W}{U}, which is exactly
    // what the three lands make. They stay tapped through seat 1's turn
    // (CR 502.3 untaps only the active player's permanents) and the pool
    // empties with the step (CR 106.4).
    tap_all_mana(&mut engine, p0);
    let commander = engine
        .state()
        .zones
        .list(ZoneLocation::Command(p0))
        .first()
        .copied()
        .expect("Katara starts in the command zone");
    engine
        .apply(p0, PlayerAction::CastSpell { card: commander })
        .expect("a Forest, a Plains and an Island pay {G}{W}{U}");
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, katara_the_fearless()).is_some()
            && stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert_eq!(
        on_battlefield(&engine, p0, katara_the_fearless()),
        Some(engine.state().commanders[0][0].object),
        "the Katara on the battlefield is the commander itself, not another copy of the card"
    );
    reach_their_main_phase(&mut engine, p1);

    // Seat 1 stacks two spells without letting either resolve: the creature is
    // the control for the counter's own filter, not a second target. It goes
    // first because it wants an empty stack (CR 302.1), and the instant follows
    // while seat 1 still holds priority.
    cast_from_hand(&mut engine, p1, llanowar_elves());
    cast_with_floating(&mut engine, p1, brainstorm());
    assert!(
        on_stack(&engine, llanowar_elves()).is_some(),
        "the creature spell is waiting on the stack for seat 0's answer, underneath"
    );
    assert!(
        on_stack(&engine, brainstorm()).is_some(),
        "and so is the instant seat 1 cast on top of it while it still held priority"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
            && on_stack(e, brainstorm()).is_some()
    });
    let seat_1_hand = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats: every land seat 0 has was spent on the commander"
    );
    let spell = in_hand(&engine, p0, fierce_guardianship()).expect("the spell is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&spell),
        "a commander is standing and there is no mana anywhere: the free \
         alternative is the only way this is offerable: {:?}",
        legal.castable
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("the commander's alternative cost pays for it");

    let storm = on_stack(&engine, brainstorm()).expect("the Brainstorm is on the stack");
    let elves = on_stack(&engine, llanowar_elves()).expect("the Elves is on the stack");
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"counter target noncreature spell\" names one, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it aims it");
    assert_eq!((min, max), (1, 1), "one spell, and the counter asks once");
    assert!(
        options.contains(&storm),
        "the instant spell is a noncreature spell: {options:?}"
    );
    assert!(
        !options.contains(&elves),
        "the creature spell on the same stack is not: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![storm],
                players: vec![],
            },
        )
        .expect("the Brainstorm was one of the options the question enumerated");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the whole price was the commander: not one mana was paid"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, brainstorm()).is_some(),
        "a countered spell goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        seat_1_hand,
        "and never resolves: a Brainstorm that did would have left seat 1 a card up"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature spell was no legal target and resolved behind the counter"
    );
    assert!(
        in_graveyard(&engine, p0, fierce_guardianship()).is_some(),
        "the instant itself resolved and went to its owner's graveyard"
    );

    // The control for the free cast: the same spell on a seat that controls no
    // commander, where the printed {2}{U} is the only way in.
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[fierce_guardianship()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, quiet_artifact());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
            && on_stack(e, quiet_artifact()).is_some()
    });

    let spell = in_hand(&engine, p0, fierce_guardianship()).expect("the spell is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&spell),
        "no commander and an empty pool: the free cast is not on offer: {:?}",
        legal.castable
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Islands, three blue"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&spell),
        "with the printed {{2}}{{U}} in the pool the spell is castable: {:?}",
        legal.castable
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("three blue pays the printed {{2}}{{U}}");
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat that paid aims it");
    let countered = on_stack(&engine, quiet_artifact()).expect("the Sol Ring is on the stack");
    assert!(options.contains(&countered), "{options:?}");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![countered],
                players: vec![],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the printed {{2}}{{U}} came out of the pool"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "the paid cast counters exactly the same way"
    );
}
