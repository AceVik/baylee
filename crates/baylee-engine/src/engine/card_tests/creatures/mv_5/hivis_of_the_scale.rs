//! `cards/creatures/mv_5/hivis_of_the_scale.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, SHIVAN_DRAGON};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn hivis_of_the_scale() -> CardIndex {
    card_index("797adc7e-03a9-4221-bdaf-4c46dcb95701")
}

/// Hivis of the Scale (797adc7e-03a9-4221-bdaf-4c46dcb95701) — {3}{R}{R} 3/4:
/// "You may choose not to untap Hivis of the Scale during your untap step.
/// {T}: Gain control of target Dragon for as long as you control Hivis of the
/// Scale and Hivis of the Scale remains tapped."
#[test]
fn hivis_of_the_scale_gains_control_of_target_dragon_while_tapped() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[hivis_of_the_scale()])
        .battlefield(1, &[SHIVAN_DRAGON])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let dragon = on_battlefield(&engine, P1, SHIVAN_DRAGON).unwrap();

    activate(&mut engine, P0, hivis_of_the_scale(), 1);
    aim_at(&mut engine, P0, dragon);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(dragon).unwrap().controller, P0);
}

/// "target Dragon": only the Dragon is on the menu, not the Grizzly Bears.
#[test]
fn hivis_of_the_scale_can_only_aim_at_a_dragon() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[hivis_of_the_scale()])
        .battlefield(1, &[SHIVAN_DRAGON, GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let dragon = on_battlefield(&engine, P1, SHIVAN_DRAGON).unwrap();
    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();

    activate(&mut engine, P0, hivis_of_the_scale(), 1);
    let menu = aim_at(&mut engine, P0, dragon);

    assert!(menu.contains(&dragon), "{menu:?}");
    assert!(!menu.contains(&bears), "{menu:?}");
}

/// P0 steals the Dragon with Hivis and the game runs to P0's next untap
/// step, where Hivis may stay tapped. Returns the engine, Hivis and the
/// Dragon with the question pending.
fn to_the_untap_question() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[hivis_of_the_scale()])
        .battlefield(1, &[SHIVAN_DRAGON])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let hivis = on_battlefield(&engine, P0, hivis_of_the_scale()).unwrap();
    let dragon = on_battlefield(&engine, P1, SHIVAN_DRAGON).unwrap();
    activate(&mut engine, P0, hivis_of_the_scale(), 1);
    aim_at(&mut engine, P0, dragon);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(dragon).unwrap().controller, P0);

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: crate::choice::ChoicePrompt::LeaveTapped,
                ..
            }
        )
    });
    (engine, hivis, dragon)
}

/// "as long as ... Hivis remains tapped": untapping Hivis at the untap step
/// returns the Dragon to its owner.
#[test]
fn hivis_of_the_scale_returns_the_dragon_when_it_untaps() {
    let (mut engine, hivis, dragon) = to_the_untap_question();
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(options.contains(&hivis), "Hivis may stay tapped");

    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && e.state().turn.phase == Phase::FirstMain
    });

    assert_eq!(engine.state().object(dragon).unwrap().controller, P1);
}

/// "You may choose not to untap": leaving Hivis tapped keeps the Dragon.
#[test]
fn hivis_of_the_scale_keeps_the_dragon_while_it_stays_tapped() {
    let (mut engine, hivis, dragon) = to_the_untap_question();

    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![hivis],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && e.state().turn.phase == Phase::FirstMain
    });

    assert_eq!(engine.state().object(dragon).unwrap().controller, P0);
}
