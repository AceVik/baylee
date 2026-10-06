//! `cards/creatures/artifacts/mv_2/baleful_strix.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Emiel the Blessed: "{3}: Exile another target creature you control, then
/// return it to the battlefield under its owner's control."
///
/// The pool's first `Effect::blink` on an *activated* ability — the five
/// that existed before it hang off a spell, a trigger or a loyalty cost — so
/// the ability is pressed out of `LegalActions::abilities`, the creature is
/// named out of the choice the engine enumerates, and the blink is read off
/// the Baleful Strix's own "When this creature enters, draw a card": a
/// creature that never left the battlefield cannot enter it, so the card
/// drawn *is* the exile-and-return having happened.
///
/// The target choice carries both halves of "another target creature you
/// control" — Emiel himself is not on offer, and neither is the opponent's
/// Llanowar Elves.
///
/// The last assertion is the half `Coverage::Partial` names. The printing
/// also says "Whenever another creature you control enters, you may pay
/// {G/W}. If you do, put a +1/+1 counter on it. If it's a Unicorn, put two
/// +1/+1 counters on it instead" — and the Strix coming back *is* another
/// creature you control entering — with a fourth Forest's `{G}` still
/// floating, so the `{G/W}` was affordable and the counter is absent for a
/// reason other than the price. That ability is deliberately not written (no
/// `Effect` asks for an optional payment of a *named* hybrid cost and runs
/// the rest of the clause on the yes), so the Strix returns a bare 1/1 and
/// nothing is asked at all: `pass_until` panics on any question that is not
/// a priority, a combat declaration or a `MayDo`, and a `MayDo` answered yes
/// — which is how the ability would have had to be written with what the
/// DSL has — would have put the counter on.
#[test]
fn emiel_blinks_another_creature_you_control_and_puts_no_counter_on_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(241, forest())
        .battlefield(
            0,
            &[
                emiel_the_blessed(),
                baleful_strix(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[llanowar_elves(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let emiel = on_battlefield(&engine, p0, emiel_the_blessed()).expect("Emiel deployed");
    let strix = on_battlefield(&engine, p0, baleful_strix()).expect("the Strix waited beside him");
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("the opponent has a creature");
    assert_eq!(pt(&engine, strix), (1, 1), "a bare Strix before anything");

    // Four Forests, three of which pay the `{3}` — the fourth is there so
    // that a `{G}` is still floating when the Strix comes back, which is
    // what the printed `{G/W}` would have been paid with. Emiel's ability
    // has no `{T}` in its cost, and he taps for nothing anyway.
    tap_all_mana_but(&mut engine, p0, None);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == emiel)
        .expect("Emiel's {3} blink is offered");
    let hand_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the Forests pay the {3}");

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the blink asks which creature: {:?}", engine.pending())
    };
    assert!(
        options.contains(&strix),
        "the other creature you control is on offer: {options:?}"
    );
    assert!(
        !options.contains(&emiel),
        "\"another\" leaves Emiel himself out: {options:?}"
    );
    assert!(
        !options.contains(&elves),
        "\"you control\" leaves the opponent's Elves out: {options:?}"
    );
    assert_eq!(options.len(), 1, "and nothing else: {options:?}");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![strix],
                players: vec![],
            },
        )
        .unwrap();

    // The ability resolves, the Strix's enters-trigger goes on the stack and
    // resolves after it.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { .. }) && stack_is_empty(e)
    });

    let strix = on_battlefield(&engine, p0, baleful_strix())
        .expect("the Strix is back on the battlefield and not left in exile");
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        hand_before + 1,
        "it entered: \"When this creature enters, draw a card\" fired",
    );
    assert!(
        on_battlefield(&engine, p0, emiel_the_blessed()).is_some(),
        "and Emiel stayed where he was",
    );
    // The `// NOT SUPPORTED:` half, asserted rather than assumed: the
    // printing would have offered `{G/W}` for a +1/+1 counter as the Strix
    // came back, and this build offers nothing and places none.
    assert_eq!(
        pt(&engine, strix),
        (1, 1),
        "the unwritten enters-trigger put no +1/+1 counter on it",
    );
}
