//! `cards/creatures/mv_3/nettling_imp.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Nettling Imp: "Activate only during an opponent's turn, before
/// attackers are declared." Offered to p1 through p0's beginning of
/// combat, refused from the declare attackers step on, and never offered
/// at all on p1's own turn, however early.
#[allow(clippy::too_many_lines)] // One printed card, played end to end.
#[test]
fn nettling_imp_is_offered_only_before_p0_declares_attackers_and_never_on_p1_s_own_turn() {
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        // A legal target throughout: without one, the ability has nothing
        // it could name (CR 601.2c, applied to activations by CR 602.2b) and
        // is withheld for that reason alone, which would say nothing about
        // the window this test is about.
        .battlefield(0, &[llanowar_elves()])
        .battlefield(1, &[nettling_imp()])
        .start();
    keep_mulligans(&mut engine);

    let imp = on_battlefield(&engine, p1, nettling_imp()).expect("the Imp is seated");
    let offered = |e: &Engine<RegistryLookup>| priority_offer(e).abilities.contains(&(imp, 0));

    // Turn 1 is p0's — an opponent's turn for p1 — and open until attackers
    // are declared.
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    assert!(
        offered(&engine),
        "p0's upkeep, before attackers are declared"
    );

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    assert!(
        offered(&engine),
        "p0's main phase, before attackers are declared"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::CombatBegin
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    assert!(
        offered(&engine),
        "beginning of combat, still before attackers"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareAttackers
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    assert!(
        !offered(&engine),
        "the declare attackers step has begun — CR 506.7a reads that as \
         the window closing regardless of whether the Elf p0 could attack \
         with was actually declared"
    );
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::ActivateAbility {
                    source: imp,
                    ability_index: 0,
                },
            )
            .is_err(),
        "the window is the engine's rule, not only the offer's"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::End
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    assert!(!offered(&engine), "the end step, later still");

    // Turn 2 is p1's own — never a legal window regardless of step.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 2
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    assert!(!offered(&engine), "p1's own upkeep");

    pass_until(&mut engine, |e| {
        e.state().turn.number == 2
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    assert!(!offered(&engine), "p1's own main phase");

    pass_until(&mut engine, |e| {
        e.state().turn.number == 2
            && e.state().turn.step == Step::CombatBegin
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    assert!(
        !offered(&engine),
        "p1's own beginning of combat: still before attackers, but not an opponent's turn"
    );
}

/// Nettling Imp's delayed destruction claims only the creature it named:
/// tapped after the ability resolved, that creature cannot obey "attacks
/// this turn if able" and is destroyed as the next end step begins, while
/// a bystander that also never attacked — because the Imp never named it
/// — is untouched.
#[test]
fn nettling_imp_destroys_only_its_named_target_when_it_could_not_attack() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[llanowar_elves(), grizzly_bears()])
        .battlefield(1, &[nettling_imp()])
        .start();
    keep_mulligans(&mut engine);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is seated");
    let bear = on_battlefield(&engine, p0, grizzly_bears()).expect("the Bear is seated");
    let imp = on_battlefield(&engine, p1, nettling_imp()).expect("the Imp is seated");

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: imp,
                ability_index: 0,
            },
        )
        .expect("offered in p0's main phase, before attackers are declared");
    aim_at(&mut engine, p1, elf);
    pass_until(&mut engine, stack_is_empty);

    // Tapped after the forced-attack effect is already in place, the Elf
    // cannot obey it — the same board state a summoning-sick or defending
    // creature would show up with.
    engine
        .dev_state_mut(p0)
        .expect("a test seat has dev commands")
        .set_tapped(elf, true);
    engine.refresh_offer();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { required, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert!(
        required.is_empty(),
        "tapped, the Elf is not able to attack, so nothing is required of it any more"
    );

    // `pass_until` declares empty attackers and blockers on the way for us.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert_eq!(
        on_battlefield(&engine, p0, llanowar_elves()),
        Some(elf),
        "combat is over and the end step has not begun yet: not yet claimed"
    );
    assert_eq!(
        on_battlefield(&engine, p0, grizzly_bears()),
        Some(bear),
        "same reading for the bystander"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::End
            && stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "it did not attack this turn, and the Imp's delayed destruction claims it"
    );
    assert_eq!(
        on_battlefield(&engine, p0, grizzly_bears()),
        Some(bear),
        "the Bear beside it also never attacked, but the Imp never named \
         it — the destruction is not a board-wide sweep"
    );
}
