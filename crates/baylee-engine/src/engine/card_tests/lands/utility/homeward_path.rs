//! `cards/lands/utility/homeward_path.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Homeward Path` is a utility land under `Coverage::Implemented`.
/// It prints "{T}: Add {C}." and "{T}: Each player gains control of all creatures they own."
/// When a creature owned by player 0 is controlled by player 1, activating `Homeward Path`'s
/// second ability restores control of the creature to player 0.
///
/// Player 1 holds the Elf through a control effect until end of turn, a Threaten's, and not
/// through its projected controller written by hand: the next refresh of the projection would
/// undo that on its own. The owner's control is a later effect, which wins (CR 613.7).
#[test]
fn homeward_path_restores_creature_control_to_owner() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[homeward_path(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf deployed");
    {
        let state = engine.dev_state_mut(p0).expect("harness sets up board");
        let filter = crate::effects::EffectFilter::object(state, elf);
        let timestamp = state.next_timestamp();
        state.effects.register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: None,
            controller: p1,
            origin: crate::effects::EffectOrigin::Resolution,
            layer: baylee_cards_dsl::Layer::Control,
            timestamp,
            duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
            filter,
            modifier: baylee_cards_dsl::Modifier::GainControl,
        });
        state.refresh_characteristics();
    }
    engine.refresh_offer();

    assert_eq!(
        engine
            .state()
            .object(elf)
            .expect("object exists")
            .controller,
        p1,
        "creature is temporarily under opponent's control"
    );

    activate(&mut engine, p0, homeward_path(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(elf)
            .expect("object exists")
            .controller,
        p0,
        "creature returned to owner's control"
    );
    let path = on_battlefield(&engine, p0, homeward_path()).expect("Homeward Path on battlefield");
    assert!(is_tapped(&engine, path), "Homeward Path is tapped");
}
