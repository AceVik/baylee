//! `cards/sorceries/mv_4/ritual_of_the_machine.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{BOG_IMP, GRIZZLY_BEARS, SAVANNAH_LIONS};

fn ritual_of_the_machine() -> CardIndex {
    card_index("bccd8298-e056-4b91-b090-408c75467b92")
}

/// Ritual of the Machine ({2}{B}{B} Sorcery):
/// - "As an additional cost to cast this spell, sacrifice a creature."
/// - "Gain control of target nonartifact, nonblack creature."
#[test]
fn ritual_of_the_machine_sacrifices_creature_and_gains_control_of_nonartifact_nonblack_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), SAVANNAH_LIONS])
        .battlefield(1, &[GRIZZLY_BEARS, BOG_IMP, ornithopter()])
        .hand(0, &[ritual_of_the_machine()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lions = on_battlefield(&engine, p0, SAVANNAH_LIONS).unwrap();
    let bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    let imp = on_battlefield(&engine, p1, BOG_IMP).unwrap();
    let thopter = on_battlefield(&engine, p1, ornithopter()).unwrap();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, ritual_of_the_machine());

    // Black creatures and artifact creatures cannot be targeted.
    for illegal in [imp, thopter] {
        assert!(
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![illegal],
                        players: vec![],
                    },
                )
                .is_err()
        );
    }

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![lions],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, SAVANNAH_LIONS).is_some());
    assert_eq!(engine.state().object(bears).unwrap().controller, p0);
}
