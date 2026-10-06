//! `cards/instants/mv_2/lose_focus.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lose Focus: "Counter target spell unless its controller pays {2}", cast
/// off exactly its own cost.
///
/// p1 casts a Dark Ritual; p0 answers with Lose Focus. The tax question goes
/// to the targeted spell's controller (p1). When p1 declines, the Ritual is
/// countered. The mana pool after resolution proves the Ritual never added
/// its {B}{B}{B}. Two Islands pay {1}{U} and not one {U} more, so replicate
/// (CR 702.56a) is never asked about — the question after the cast is the
/// target — and a cost paid no times triggers nothing (CR 603.4): two spells
/// on the stack and no copy.
#[test]
fn lose_focus_counters_the_targeted_spell_when_the_controller_declines_to_pay_two() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(29, forest())
        .battlefield(0, &[island(), island()])
        .hand(0, &[lose_focus()])
        .battlefield(1, &[swamp()])
        .hand(1, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);

    reach_main_phase(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    // p1 casts Dark Ritual.
    let ritual = in_hand(&engine, p1, dark_ritual()).expect("the Ritual is in hand");
    cast_from_hand(&mut engine, p1, dark_ritual());

    // p0 taps both Islands and counters with Lose Focus.
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p0, lose_focus());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("Lose Focus asks for a target, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "their spell, their target choice");
    assert!(
        options.contains(&ritual),
        "the Ritual on the stack is a legal target: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ritual],
            },
        )
        .expect("the Ritual is a legal target");

    // No replicate paid, so no trigger: two spells and nothing above them.
    let stack = engine.state().zones.list(ZoneLocation::Stack).clone();
    assert_eq!(
        stack.len(),
        2,
        "the Ritual and Lose Focus are on the stack and nothing else: the \
         replicate cost was paid no times, so its trigger does not trigger: \
         {stack:?}"
    );

    // Both pass; Lose Focus resolves and the tax is offered to p1.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayTax { mana },
        ..
    } = engine.pending()
    else {
        unreachable!("pass_until stopped on the tax question")
    };
    assert_eq!(*mana, 2, "Lose Focus prints a {{2}} tax");
    assert_eq!(
        *player, p1,
        "\"unless its controller pays\" — the tax belongs to the targeted spell's controller"
    );

    // p1 declines — the Ritual is countered.
    engine
        .apply(p1, PlayerAction::YesNo(false))
        .expect("declining is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, dark_ritual()).is_some(),
        "the Ritual was countered and goes to p1's graveyard"
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "the tax went unpaid and the Ritual was countered — no mana floats"
    );
}

/// Lose Focus with its replicate cost paid once (CR 702.56a): "When you cast
/// this spell, if a replicate cost was paid for it, copy it for each time
/// its replicate cost was paid. If the spell has any targets, you may choose
/// new targets for any of the copies."
///
/// p1 has two Dark Rituals on the stack and p0 four Islands floating. The
/// question after the cast counts replicate payments and offers two, which
/// is what {U}{U}{U}{U} pays beside {1}{U}; p0 pays once and aims at the
/// second Ritual. The trigger goes on the stack above the spell, and as it
/// resolves the copy is put there aimed at that Ritual too; p0 turns it onto
/// the first (CR 707.10c). Each asks p1 for {2}, p1 has nothing floating,
/// and both Rituals are countered. The copy was never cast — three casts in
/// the journal, not four — and ceases to exist as it leaves the stack
/// (CR 704.5e), so one Lose Focus lies in the graveyard. The {U} left
/// floating is the fourth Island: the payment took {1}{U} and one {U}.
#[test]
#[allow(clippy::too_many_lines)] // one game: two casts, the copy, and both taxes
fn lose_focus_replicated_once_counters_a_second_spell_with_its_copy() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(29, forest())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[lose_focus()])
        .battlefield(1, &[swamp(), swamp()])
        .hand(1, &[dark_ritual(), dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    // p1 casts both Rituals, the second while holding priority.
    let first = in_hand(&engine, p1, dark_ritual()).expect("a Ritual in hand");
    cast_from_hand(&mut engine, p1, dark_ritual());
    let second = in_hand(&engine, p1, dark_ritual()).expect("the other Ritual");
    cast_with_floating(&mut engine, p1, dark_ritual());
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    let spell = in_hand(&engine, p0, lose_focus()).expect("Lose Focus in hand");
    cast_lose_focus_replicated(&mut engine, p0, 2, 1, second);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "{{1}}{{U}} and one {{U}} paid out of four: the replicate cost is \
         part of the total cost (CR 601.2f)"
    );
    let trigger = replicate_trigger(&engine, spell).expect("paid once, so it triggers");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Stack).last(),
        Some(&trigger),
        "the trigger goes on the stack above the spell it copies"
    );

    // Both pass: the trigger resolves and puts the copy on the stack, whose
    // controller may change its target (CR 707.10c).
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("the copy may take new targets, got {:?}", engine.pending())
    };
    assert_eq!(
        player, p0,
        "the copy is controlled by the trigger's controller"
    );
    assert_eq!((min, max), (0, 1), "keeping the target is an answer");
    assert!(
        options.contains(&first) && !options.contains(&second),
        "the first Ritual is another target; the second is the one it has: \
         {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![first],
            },
        )
        .expect("the first Ritual is a legal new target");

    let stack = engine.state().zones.list(ZoneLocation::Stack).clone();
    assert_eq!(
        stack.len(),
        4,
        "two Rituals, Lose Focus and its copy: {stack:?}"
    );
    let copy = *stack.last().expect("the copy is on top");
    let copied = engine.state().object(copy).expect("the copy exists");
    assert_eq!(copied.controller, p0);
    assert_eq!(
        copied.targets.as_slice(),
        &[first],
        "aimed where p0 turned it"
    );
    assert!(copied.riders.contains(&crate::object::Rider::SpellCopy));
    assert!(
        !copied.cast_from_hand,
        "a copy is put on the stack, not cast"
    );
    let casts = engine
        .state()
        .journal
        .entries()
        .iter()
        .filter(|e| matches!(e.event, crate::event::GameEvent::SpellCast { .. }))
        .count();
    assert_eq!(
        casts, 3,
        "two Rituals and Lose Focus were cast; the copy was not"
    );

    // The copy resolves first and taxes p1 for the first Ritual, then the
    // spell does for the second. p1 has nothing floating and declines both.
    for _ in 0..2 {
        pass_until(&mut engine, |e| {
            matches!(
                e.pending(),
                Pending::YesNo {
                    prompt: YesNoPrompt::PayTax { .. },
                    ..
                }
            )
        });
        let Pending::YesNo { player, .. } = engine.pending().clone() else {
            unreachable!("pass_until stopped on the tax")
        };
        assert_eq!(player, p1, "each asks the targeted spell's controller");
        engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    let graveyard = |seat| {
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(seat))
            .clone()
    };
    assert!(
        graveyard(p1).contains(&first) && graveyard(p1).contains(&second),
        "both Rituals were countered: {:?}",
        graveyard(p1)
    );
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);
    assert_eq!(
        graveyard(p0),
        vec![spell],
        "one Lose Focus in the graveyard: the copy ceased to exist"
    );
}

/// A replicated Lose Focus countered in response to its own trigger still
/// makes its copy.
///
/// "Once triggered, an ability exists on the stack independently of its
/// source" (CR 113.7a), and "copy it" asks about a spell no longer where the
/// effect expected it, so the copy is made from the spell as it last
/// existed (CR 608.2h). p0 replicates once at the Ritual; p1 answers the
/// trigger with a Lose Focus of their own aimed at p0's, and p0 cannot pay.
/// The trigger then resolves over a countered spell, and its copy still
/// counters the Ritual.
#[test]
fn a_countered_lose_focus_still_copies_itself() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(31, forest())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[lose_focus()])
        .battlefield(1, &[swamp(), island(), island()])
        .hand(1, &[dark_ritual(), lose_focus()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    let ritual = in_hand(&engine, p1, dark_ritual()).expect("the Ritual in hand");
    cast_from_hand(&mut engine, p1, dark_ritual());
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    let spell = in_hand(&engine, p0, lose_focus()).expect("Lose Focus in hand");
    cast_lose_focus_replicated(&mut engine, p0, 1, 1, ritual);
    assert!(replicate_trigger(&engine, spell).is_some());
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    // p1 answers the trigger: {U}{U} floating pays {1}{U} and no replicate,
    // so the next question is the target.
    cast_with_floating(&mut engine, p1, lose_focus());
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![spell],
            },
        )
        .expect("p0's Lose Focus is a legal target");
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    assert!(
        in_graveyard(&engine, p0, lose_focus()).is_some(),
        "p0's Lose Focus was countered under its own trigger"
    );

    // The trigger resolves anyway: the copy is made and taxes p1.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the tax")
    };
    assert_eq!(player, p1, "the copy counters the Ritual unless p1 pays");
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, dark_ritual()).is_some(),
        "the copy of the countered spell countered the Ritual"
    );
}

/// Lose Focus replicated eighteen times off twenty Islands: "you may pay
/// [cost] any number of times" (CR 702.56a), and every payment is part of
/// the total cost (CR 601.2f).
///
/// The fuzzer's panic (2026-09-29), on the engine's own path. A cost was a
/// list of sixteen symbols, and the replicate question's bound priced one
/// payment more at a time before it asked whether the pool paid: once
/// fourteen payments were payable (sixteen mana), pricing the fifteenth made
/// `{1}{U}` and fifteen `{U}`, seventeen symbols, and the list asserted and
/// took the game down. The payment and the trigger's copies are counted the
/// same way, so all eighteen are paid and all eighteen copies are made.
#[test]
fn lose_focus_replicated_eighteen_times_is_paid_and_copied_eighteen_times() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[island(); 20])
        .hand(0, &[lose_focus()])
        .battlefield(1, &[swamp()])
        .hand(1, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    let ritual = in_hand(&engine, p1, dark_ritual()).expect("the Ritual in hand");
    cast_from_hand(&mut engine, p1, dark_ritual());
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    let spell = in_hand(&engine, p0, lose_focus()).expect("Lose Focus in hand");
    cast_lose_focus_replicated(&mut engine, p0, 18, 18, ritual);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{U}} and eighteen {{U}}: all twenty Islands paid"
    );
    assert_eq!(
        engine.state().object(spell).map(|o| o.replicated),
        Some(18),
        "the spell remembers every payment"
    );
    assert!(
        replicate_trigger(&engine, spell).is_some(),
        "paid, so it triggers"
    );

    // Both pass: the trigger resolves into eighteen copies, and each may take
    // a new target (CR 707.10c). Each keeps the Ritual.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    for copy in 1..=18 {
        let Pending::ChooseTargets { player, .. } = engine.pending().clone() else {
            panic!(
                "copy {copy} may take a new target, got {:?}",
                engine.pending()
            )
        };
        assert_eq!(player, p0, "the trigger's controller controls the copies");
        engine
            .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
            .expect("keeping the target is an answer");
    }
    let stack = engine.state().zones.list(ZoneLocation::Stack).clone();
    assert_eq!(
        stack.len(),
        20,
        "the Ritual, Lose Focus and eighteen copies: {stack:?}"
    );
    assert_eq!(
        stack
            .iter()
            .filter(|id| engine
                .state()
                .object(**id)
                .is_some_and(|o| o.riders.contains(&crate::object::Rider::SpellCopy)))
            .count(),
        18,
        "every payment made its copy"
    );
}

/// Lose Focus with mana for a replicate payment and none made: "if a
/// replicate cost was paid for it" is an intervening "if" (CR 603.4), so
/// nothing triggers, and the {U} not spent stays in the pool.
#[test]
fn lose_focus_replicated_no_times_triggers_nothing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(37, forest())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[lose_focus()])
        .battlefield(1, &[swamp()])
        .hand(1, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    let ritual = in_hand(&engine, p1, dark_ritual()).expect("the Ritual in hand");
    cast_from_hand(&mut engine, p1, dark_ritual());
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    let spell = in_hand(&engine, p0, lose_focus()).expect("Lose Focus in hand");
    cast_lose_focus_replicated(&mut engine, p0, 1, 0, ritual);
    assert_eq!(
        replicate_trigger(&engine, spell),
        None,
        "paid no times, nothing triggers"
    );
    assert_eq!(engine.state().zones.list(ZoneLocation::Stack).len(), 2);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{U}} not paid for replicate is still floating"
    );
}
