//! `cards/enchantments/auras/mv_4/abduction.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

fn abduction() -> CardIndex {
    card_index("d0e1904e-1a37-41f6-8582-b9ea794bb886")
}

/// `Abduction` ({2}{U}{U}, Enchantment — Aura) untaps enchanted creature on
/// entry, gains control of it, and returns it to the battlefield under its
/// owner's control when it dies.
#[test]
fn abduction_untaps_gains_control_and_returns_to_owner_on_death() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), mountain()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[abduction(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    engine
        .dev_state_mut(p0)
        .unwrap()
        .object_mut(bears)
        .unwrap()
        .status
        .insert(Status::TAPPED);

    cast_from_hand(&mut engine, p0, abduction());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !engine
            .state()
            .object(bears)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    assert_eq!(engine.state().object(bears).unwrap().controller, p0);

    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, GRIZZLY_BEARS).is_some());
}

/// "When *enchanted* creature dies": another creature of the same owner that
/// dies stays in the graveyard, and the stolen Bears stay stolen.
#[test]
fn abduction_returns_only_the_enchanted_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), mountain()])
        .battlefield(1, &[GRIZZLY_BEARS, HILL_GIANT])
        .hand(0, &[abduction(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, p0, abduction());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(bears).unwrap().controller, p0);

    cast_with_floating(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, HILL_GIANT).is_some());
    assert!(
        on_battlefield(&engine, p1, HILL_GIANT).is_none(),
        "a creature that was not enchanted does not come back"
    );
    assert_eq!(
        engine.state().object(bears).unwrap().controller,
        p0,
        "the enchanted Bears are still ours"
    );
}
