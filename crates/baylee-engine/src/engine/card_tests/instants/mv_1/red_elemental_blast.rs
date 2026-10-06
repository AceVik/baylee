//! `cards/instants/mv_1/red_elemental_blast.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Red Elemental Blast, mode one: "Counter target spell if it's blue." Read
/// off an Unsummon, cast in response before it can bounce anything.
#[test]
fn red_elemental_blast_mode_0_counters_a_blue_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[quiet_creature(), mountain()])
        .hand(0, &[red_elemental_blast()])
        .battlefield(1, &[island(), oboro_envoy()])
        .hand(1, &[unsummon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // A blue permanent stands beside the blue spell so both modes have a
    // legal target and the modal choice is really asked.
    assert!(
        on_battlefield(&engine, p1, oboro_envoy()).is_some(),
        "seated"
    );
    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("seated");
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, unsummon());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .expect("the Elf is a legal target");
    let bounce = on_stack(&engine, unsummon()).expect("on the stack");

    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, red_elemental_blast());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected a mode choice, got {:?}", engine.pending())
    };
    let slot = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(0)))
        .expect("mode 0 offered");
    engine.apply(p0, PlayerAction::ChooseMode(slot)).unwrap();
    let Pending::ChooseTargets {
        options: targets, ..
    } = engine.pending().clone()
    else {
        panic!("expected a spell target, got {:?}", engine.pending())
    };
    assert!(
        targets.contains(&bounce),
        "a blue spell is a legal target: {targets:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bounce],
            },
        )
        .expect("Unsummon is blue");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_some(),
        "countered before it could bounce the Elf"
    );
    assert!(
        in_graveyard(&engine, p1, unsummon()).is_some(),
        "a countered spell still moves to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, oboro_envoy()).is_some(),
        "mode 0 was chosen — the other mode's candidate is untouched"
    );
}

/// Red Elemental Blast, mode two: "Destroy target permanent if it's blue."
#[test]
fn red_elemental_blast_mode_1_destroys_a_blue_permanent() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain()])
        .hand(0, &[red_elemental_blast()])
        .battlefield(1, &[island(), oboro_envoy(), quiet_creature()])
        .hand(1, &[unsummon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // A blue spell on the stack beside the blue permanent, so both modes
    // have a legal target and the modal choice is really asked. Aimed at
    // p1's own Elf, out of the way of this test.
    let envoy = on_battlefield(&engine, p1, oboro_envoy()).expect("seated");
    let elf = on_battlefield(&engine, p1, quiet_creature()).expect("seated");
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, unsummon());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .expect("p1 may bounce its own Elf");
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, red_elemental_blast());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected a mode choice, got {:?}", engine.pending())
    };
    let slot = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(1)))
        .expect("mode 1 offered");
    engine.apply(p0, PlayerAction::ChooseMode(slot)).unwrap();
    let Pending::ChooseTargets {
        options: targets, ..
    } = engine.pending().clone()
    else {
        panic!("expected an object target, got {:?}", engine.pending())
    };
    assert!(
        targets.contains(&envoy),
        "a blue permanent is a legal target: {targets:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![envoy],
            },
        )
        .expect("Oboro Envoy is blue");
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p1, oboro_envoy()).is_none());
    assert!(in_graveyard(&engine, p1, oboro_envoy()).is_some());
}
