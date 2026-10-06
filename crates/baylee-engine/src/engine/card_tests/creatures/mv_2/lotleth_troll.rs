//! `cards/creatures/mv_2/lotleth_troll.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lotleth Troll — {B}{G} 2/1 Zombie Troll with trample — and the whole of
/// what the engine writes of it: "Discard a creature card: Put a +1/+1
/// counter on this creature."
///
/// Every half of that sentence is the *engine's* answer and not the card's,
/// so the test reads it out of the question it is asked. The menu holds the
/// Llanowar Elves and none of the seven Forests the filler deck dealt, which
/// is `Filter::CREATURE` doing its work; the prompt variant is what says the
/// card is being *paid* and not searched for; and the payoff is counted on
/// the battleffeld rather than read off the card file — one +1/+1 counter
/// turns the printed 2/1 into a 3/2, which no other reading of the board
/// produces.
///
/// The refused follow-up is the cost's other half. With the only creature
/// card spent, no creature card is left in hand, the cost cannot be paid
/// (CR 118.3) and the engine stops offering the ability at all — which is
/// how this engine refuses every cost a board cannot meet. And the `{B}`
/// regeneration standing beside that refusal is the point of the floating
/// black: two abilities priced in different currencies, one of which the
/// board can no longer afford and one of which it still can.
#[test]
// One scenario and not two: the second offer is worth reading only *after*
// the first has been spent, so splitting it would mean building the same
// board twice to assert half of it each time.
#[allow(clippy::too_many_lines)]
fn lotleth_troll_trades_a_creature_card_for_a_counter_and_regenerates() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(17, forest())
        // A third land, and it is the {B} the regenerate line would cost.
        // `cast_from_hand` taps everything and the Troll takes {B}{G}, so
        // one black is left floating when the offer below is read — without
        // it `can_afford` refuses a priced ability whatever the card says,
        // and the pin would keep passing after the line was written.
        .battlefield(0, &[forest(), swamp(), swamp()])
        .hand(0, &[lotleth_troll(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, lotleth_troll());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let troll = on_battlefield(&engine, p0, lotleth_troll()).expect("the Troll resolved");
    assert!(
        keywords(&engine, troll).contains(KeywordSet::TRAMPLE),
        "trample is the card's whole keyword line"
    );
    assert_eq!(
        pt(&engine, troll),
        (2, 1),
        "a printed 2/1 with nothing on it yet"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let offered = deeds(&legal, &[troll]);
    assert!(
        matches!(offered[..], [(0, Deed::Ability(0)), (0, Deed::Ability(1))]),
        "both printed abilities are offered: the discard because a creature \
         card is there to pay it, the regeneration because a black is \
         floating: {offered:?}"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "and that {{B}} is the regeneration's whole price"
    );

    activate(&mut engine, p0, lotleth_troll(), 0);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("the cost asks which card, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat pays the cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostDiscard,
        "the variant is what tells a client this is a cost and not a search"
    );
    assert_eq!((min, max), (1, 1), "one card, and the cost asks once");
    let fodder = in_hand(&engine, p0, llanowar_elves()).expect("the Elves are in hand");
    assert_eq!(
        options,
        vec![fodder],
        "a creature card and nothing else: the seven Forests the filler deck \
         dealt are no more discardable to this cost than the opponent's board"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the creature card the question offered pays the cost");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_none(),
        "the discarded card left the hand"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "and is in its owner's graveyard"
    );
    assert_eq!(
        counters_on(&engine, troll, CounterKind::P1P1),
        1,
        "one +1/+1 counter, put on the creature the ability names"
    );
    assert_eq!(
        pt(&engine, troll),
        (3, 2),
        "so the 2/1 is a 3/2 — and the Elves' absence from the board is the \
         other half of the same counter"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(troll, 0)),
        "with the only creature card spent, the cost cannot be paid and the \
         ability is no longer offered: {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.contains(&(troll, 1)),
        "the regeneration is priced in mana and the {{B}} never moved, so it \
         is still there: {:?}",
        legal.abilities
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: troll,
                ability_index: 1,
            },
        )
        .expect("the floating {B} pays for it");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        engine
            .state()
            .object(troll)
            .expect("the Troll is where it was")
            .regeneration_shields,
        1,
        "`Regenerate this creature` needs no target and shields itself"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        0,
        "and the black that had been floating all along is what paid"
    );
}
