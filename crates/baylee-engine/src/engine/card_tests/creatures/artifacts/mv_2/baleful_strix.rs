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
/// Declining the optional hybrid payment leaves the returning Strix unchanged.
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
    aim_at(&mut engine, p0, strix);

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    // Resolve the Strix's draw trigger as well.
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
    assert_eq!(
        pt(&engine, strix),
        (1, 1),
        "declining the hybrid payment puts no counter on it",
    );
}
