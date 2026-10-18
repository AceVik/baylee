//! `cards/enchantments/mv_3/oubliette.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn oubliette_source_phasing_is_not_leaving_before_resolution() {
    use baylee_core::generated::index::OUBLIETTE;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for leaves in [false, true] {
        let mut engine = Duel::new(SEED, swamp())
            .battlefield(0, &[swamp(), swamp(), swamp()])
            .hand(0, &[OUBLIETTE])
            .battlefield(1, &[grizzly_bears()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
        cast_from_hand(&mut engine, p0, OUBLIETTE);
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseTargets { .. })
        });
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![bear],
                },
            )
            .unwrap();
        let source = on_battlefield(&engine, p0, OUBLIETTE).unwrap();
        let state = engine.dev_state_mut(p0).unwrap();
        if leaves {
            state
                .move_object(
                    source,
                    ZoneLocation::Graveyard(p0),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
        } else {
            state.phase_out(&[source]);
        }
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            engine
                .state()
                .object(bear)
                .unwrap()
                .status
                .contains(crate::object::Status::PHASED_OUT),
            !leaves
        );
    }
}

#[test]
fn oubliette_phases_target_out_across_untap_and_phases_it_in_tapped_when_oubliette_leaves() {
    let oubliette = baylee_core::generated::index::OUBLIETTE;
    let disenchant = baylee_core::generated::index::DISENCHANT;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[oubliette])
        .battlefield(1, &[grizzly_bears(), plains(), plains()])
        .hand(1, &[disenchant])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p1, grizzly_bears()).expect("target creature on table");

    cast_from_hand(&mut engine, p0, oubliette);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bear],
            },
        )
        .expect("target creature for Oubliette ETB trigger");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(bear)
            .expect("bear object")
            .status
            .contains(crate::object::Status::PHASED_OUT),
        "Oubliette's ETB trigger phases the target creature out"
    );

    // Advance into p1's main phase (through p1's untap step): CR 610.4 says a
    // permanent phased out until an event occurs does not phase in during its
    // controller's untap step.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        engine
            .state()
            .object(bear)
            .expect("bear object")
            .status
            .contains(crate::object::Status::PHASED_OUT),
        "creature phased out by Oubliette does not phase in during untap step"
    );

    let oubliette_id = on_battlefield(&engine, p0, oubliette).expect("Oubliette on table");
    cast_from_hand(&mut engine, p1, disenchant);
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![oubliette_id],
            },
        )
        .expect("Disenchant targets Oubliette");
    pass_until(&mut engine, stack_is_empty);

    let bear_obj = engine.state().object(bear).expect("bear object");
    assert!(
        !bear_obj.status.contains(crate::object::Status::PHASED_OUT),
        "creature phases back in immediately when Oubliette leaves the battlefield"
    );
    assert!(
        bear_obj.status.contains(crate::object::Status::TAPPED),
        "creature is tapped as it phases in this way"
    );
}
