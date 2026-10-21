//! Giant Oyster — `{2}{U}{U}` 0/3 Creature — Oyster (Homelands)
//!
//! You may choose not to untap this creature during your untap step.
//! `{T}`: For as long as this creature remains tapped, target tapped creature
//! doesn't untap during its controller's untap step, and at the beginning of
//! each of your draw steps, put a -1/-1 counter on that creature. When this
//! creature leaves the battlefield or becomes untapped, remove all -1/-1
//! counters from the creature.
//!
//! (`Coverage::Partial`: recurring draw-step -1/-1 counters on the linked
//! creature and removing them when Giant Oyster leaves or untaps are not in
//! `Effect`; may choose not to untap and `{T}` keeps target tapped creature
//! from untapping for as long as Giant Oyster remains tapped.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn giant_oyster() -> CardIndex {
    card_index("062b4d84-5ba3-417b-b8e1-65afce7b05e3")
}

fn bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

#[test]
fn giant_oyster_keeps_target_tapped_creature_from_untapping_while_oyster_stays_tapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[giant_oyster()])
        .battlefield(1, &[bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let oyster = on_battlefield(&engine, p0, giant_oyster()).unwrap();
    let target_bears = on_battlefield(&engine, p1, bears()).unwrap();

    // p1 attacks with Grizzly Bears so it becomes tapped.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(target_bears, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && e.state().turn.phase == Phase::SecondMain
            && stack_is_empty(e)
    });
    assert!(is_tapped(&engine, target_bears));

    // On p0's main phase, activate Giant Oyster (ability_index: 1) targeting tapped Grizzly Bears.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: oyster,
                ability_index: 1,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![target_bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Advance to p1's next FirstMain: Grizzly Bears did NOT untap because Giant Oyster is still tapped.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        is_tapped(&engine, target_bears),
        "target tapped creature stays tapped while Giant Oyster remains tapped"
    );

    // On p0's next untap step, p0 chooses to leave Giant Oyster tapped via LeaveTapped.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: crate::choice::ChoicePrompt::LeaveTapped,
                ..
            }
        )
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![oyster],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        is_tapped(&engine, target_bears),
        "target tapped creature still stays tapped when Giant Oyster chooses not to untap"
    );

    // On p0's next turn, pass_until answers LeaveTapped with vec![], so Giant Oyster untaps and Bears untap on p1's turn!
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(!is_tapped(&engine, oyster));
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        !is_tapped(&engine, target_bears),
        "once Giant Oyster untaps, the target creature untaps on its controller's untap step"
    );
}
