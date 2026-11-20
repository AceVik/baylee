//! `cards/creatures/artifacts/mv_3/proteus_machine.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::subtypes;

fn proteus_machine() -> CardIndex {
    card_index("c3acf94e-3adc-4397-a736-3f6072a5967d")
}

/// Proteus Machine may be cast face down for `{3}` via morph, turned face up
/// for `{0}`, and then becomes the chosen creature type indefinitely (replacing
/// its printed Shapeshifter subtype rather than wearing off at cleanup).
#[test]
fn proteus_machine_morphs_for_zero_and_becomes_chosen_type_indefinitely() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[proteus_machine()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, proteus_machine());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("normal and morph modes expected: {:?}", engine.pending());
    };
    let morph = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Disguise))
        .expect("morph cast mode offered");
    engine
        .apply(p0, PlayerAction::ChooseMode(morph))
        .expect("cast Proteus Machine face down");
    pass_until(&mut engine, stack_is_empty);

    let machine =
        on_battlefield(&engine, p0, proteus_machine()).expect("face-down permanent seated");
    assert!(
        engine
            .state()
            .object(machine)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN),
        "entered face down"
    );
    assert_eq!(types(&engine, machine), TypeSet::CREATURE);
    assert!(
        engine
            .state()
            .object(machine)
            .unwrap()
            .characteristics()
            .subtypes
            .is_empty(),
        "face-down creature has no subtypes"
    );

    // Turn face up via morph {0}.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: machine,
                ability_index: baylee_engine::choice::TURN_FACE_UP,
            },
        )
        .expect("turn Proteus Machine face up for {0}");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::DRAGON))
        .expect("choose Dragon on turned-face-up trigger");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let chars = &engine.state().object(machine).unwrap().characteristics();
    assert!(
        !engine
            .state()
            .object(machine)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert!(chars.types.contains(TypeSet::ARTIFACT));
    assert!(chars.types.contains(TypeSet::CREATURE));
    assert!(
        chars.subtypes.contains(subtypes::creature::DRAGON),
        "became chosen type Dragon"
    );
    assert!(
        !chars.subtypes.contains(subtypes::creature::SHAPESHIFTER),
        "replaced Shapeshifter instead of adding"
    );

    // Advance across opponent's turn and back to P0's next turn: the type change
    // is indefinite, so Dragon persists past cleanup.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    let chars_later = &engine.state().object(machine).unwrap().characteristics();
    assert!(chars_later.subtypes.contains(subtypes::creature::DRAGON));
    assert!(
        !chars_later
            .subtypes
            .contains(subtypes::creature::SHAPESHIFTER)
    );
}
