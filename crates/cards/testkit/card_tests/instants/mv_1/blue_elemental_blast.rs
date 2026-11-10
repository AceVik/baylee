//! `cards/instants/mv_1/blue_elemental_blast.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Blue Elemental Blast, mode one: "Counter target spell if it's red." Read
/// off a Lightning Bolt, cast in response to it before it can deal damage.
#[test]
fn blue_elemental_blast_mode_0_counters_a_red_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island()])
        .hand(0, &[blue_elemental_blast()])
        .battlefield(1, &[mountain(), flame_spirit()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // A red permanent stands beside the red spell so both modes have a
    // legal target and the modal choice is really asked — with only one
    // candidate on the whole board, the engine collapses the choice.
    assert!(
        on_battlefield(&engine, p1, flame_spirit()).is_some(),
        "seated"
    );
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .expect("the Bolt at p0");
    let bolt = on_stack(&engine, lightning_bolt()).expect("on the stack");

    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, blue_elemental_blast());
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
        targets.contains(&bolt),
        "a red spell is a legal target: {targets:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bolt],
            },
        )
        .expect("the Bolt is red");

    let before = life_of(&engine, p0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        life_of(&engine, p0),
        before,
        "countered before it could deal damage"
    );
    assert!(
        in_graveyard(&engine, p1, lightning_bolt()).is_some(),
        "a countered spell still moves to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, flame_spirit()).is_some(),
        "mode 0 was chosen — the other mode's candidate is untouched"
    );
}

/// Blue Elemental Blast, mode two: "Destroy target permanent if it's red."
#[test]
fn blue_elemental_blast_mode_1_destroys_a_red_permanent() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island()])
        .hand(0, &[blue_elemental_blast()])
        .battlefield(1, &[mountain(), flame_spirit()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // A red spell on the stack beside the red permanent, so both modes have
    // a legal target and the modal choice is really asked. Aimed at its own
    // caster, out of the way of this test.
    let spirit = on_battlefield(&engine, p1, flame_spirit()).expect("seated");
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("p1 may aim the Bolt at itself");
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, blue_elemental_blast());
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
        targets.contains(&spirit),
        "a red permanent is a legal target: {targets:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![spirit],
            },
        )
        .expect("Flame Spirit is red");
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p1, flame_spirit()).is_none());
    assert!(in_graveyard(&engine, p1, flame_spirit()).is_some());
}
