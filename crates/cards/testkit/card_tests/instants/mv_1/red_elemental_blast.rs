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

/// Mode two names "blue permanent", and the menu is the restriction: the
/// opponent's blue Envoy and our own blue Envoy are on it (it says nothing
/// about control), a green Elf and a black Goblin of the opponent's are not,
/// and neither is an Island, which is a colourless land.
#[test]
fn red_elemental_blast_destroy_mode_offers_only_blue_permanents() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), oboro_envoy()])
        .hand(0, &[red_elemental_blast()])
        .battlefield(
            1,
            &[
                island(),
                oboro_envoy(),
                llanowar_elves(),
                festering_goblin(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ours = on_battlefield(&engine, p0, oboro_envoy()).expect("our blue creature");
    let theirs = on_battlefield(&engine, p1, oboro_envoy()).expect("their blue creature");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("green");
    let goblin = on_battlefield(&engine, p1, festering_goblin()).expect("black");
    let land = on_battlefield(&engine, p1, island()).expect("a colourless land");

    cast_from_hand(&mut engine, p0, red_elemental_blast());
    // With no spell on the stack only the second mode has a target, so the
    // mode question may not be asked at all.
    if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
        let slot = options
            .iter()
            .position(|o| matches!(o.kind, CastModeKind::Mode(1)))
            .expect("the destroy mode is offered");
        assert!(
            !options
                .iter()
                .any(|o| matches!(o.kind, CastModeKind::Mode(0))),
            "no spell on the stack: the counter mode has no target to be offered for"
        );
        engine.apply(p0, PlayerAction::ChooseMode(slot)).unwrap();
    }
    let menu = aim_at_objects(&mut engine, p0, theirs);
    assert!(
        menu.contains(&ours) && menu.contains(&theirs),
        "a blue permanent of either player: {menu:?}"
    );
    for refused in [elf, goblin, land] {
        assert!(!menu.contains(&refused), "not blue, not a target: {menu:?}");
    }
    assert_eq!(menu.len(), 2, "the two Envoys are the whole menu");
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p1, oboro_envoy()).is_none());
    assert!(on_battlefield(&engine, p1, llanowar_elves()).is_some());
}

/// Names the target of a choice that asks for objects and hands back the menu
/// it offered (`aim_at` answers with `ChooseTargets`; the modes of a modal
/// spell take `ChooseObjects`).
#[track_caller]
fn aim_at_objects(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    target: ObjectId,
) -> Vec<ObjectId> {
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert_eq!(player, seat);
    engine
        .apply(
            seat,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .expect("the named target was on the menu");
    options
}

/// Mode one names "blue spell": with a blue Unsummon and a green Giant Growth
/// both on the stack, the menu holds the Unsummon and not the Giant Growth,
/// and the Giant Growth is still on the stack, uncountered, afterwards.
#[test]
fn red_elemental_blast_counter_mode_offers_only_blue_spells() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), quiet_creature()])
        .hand(0, &[red_elemental_blast()])
        .battlefield(1, &[island(), forest(), llanowar_elves()])
        .hand(1, &[unsummon(), giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("seated");

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, unsummon());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![their_elf],
                players: vec![],
            },
        )
        .unwrap();
    cast_with_floating(&mut engine, p1, giant_growth());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![their_elf],
                players: vec![],
            },
        )
        .unwrap();
    let bounce = on_stack(&engine, unsummon()).expect("the blue spell");
    let growth = on_stack(&engine, giant_growth()).expect("the green spell");
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, red_elemental_blast());
    // Only the counter mode has a spell to point at, but no permanent of the
    // opponent's is blue, so either way the mode is the first one.
    if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
        let slot = options
            .iter()
            .position(|o| matches!(o.kind, CastModeKind::Mode(0)))
            .expect("the counter mode is offered");
        engine.apply(p0, PlayerAction::ChooseMode(slot)).unwrap();
    }
    let menu = aim_at_objects(&mut engine, p0, bounce);
    assert!(menu.contains(&bounce), "the blue spell: {menu:?}");
    assert!(
        !menu.contains(&growth),
        "a green spell is not a legal target: {menu:?}"
    );
    pass_until(&mut engine, |e| {
        on_stack(e, red_elemental_blast()).is_none()
    });
    assert!(
        in_graveyard(&engine, p1, unsummon()).is_some(),
        "the blue spell was countered"
    );
    assert!(
        on_stack(&engine, giant_growth()).is_some(),
        "the green spell is still waiting to resolve"
    );
}
