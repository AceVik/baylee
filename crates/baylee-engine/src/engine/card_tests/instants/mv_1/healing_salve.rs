//! `cards/instants/mv_1/healing_salve.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Healing Salve, mode one: "Target player gains 3 life."
#[test]
fn healing_salve_mode_0_gains_3_life_for_a_target_player() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains()])
        .hand(0, &[healing_salve()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let before = life_of(&engine, p0);
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, healing_salve());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected a mode choice, got {:?}", engine.pending())
    };
    let slot = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(0)))
        .expect("mode 0 is offered");
    engine.apply(p0, PlayerAction::ChooseMode(slot)).unwrap();
    engine
        .apply(p0, PlayerAction::ChoosePlayer(p0))
        .expect("p0 is a legal target player");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p0), before + 3, "\"gains 3 life\"");
}

/// Healing Salve, mode two: "Prevent the next 3 damage that would be dealt
/// to any target this turn." Read off a Lightning Bolt aimed at the same
/// player the shield stands on.
#[test]
fn healing_salve_mode_1_prevents_the_next_3_damage_to_its_target() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), mountain()])
        .hand(0, &[healing_salve(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let white = on_battlefield(&engine, p0, plains()).expect("the Plains is out");
    tap_mana_where(&mut engine, p0, |id| id == white);
    cast_with_floating(&mut engine, p0, healing_salve());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected a mode choice, got {:?}", engine.pending())
    };
    let slot = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(1)))
        .expect("mode 1 is offered");
    engine.apply(p0, PlayerAction::ChooseMode(slot)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .expect("p0 is any target");
    pass_until(&mut engine, stack_is_empty);

    let before = life_of(&engine, p0);
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .expect("p0 is any target");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p0), before, "3 dealt, 3 prevented");
}

/// Healing Salve's "prevent the next 3 damage" (mode two) shields p0 before
/// a Lightning Bolt lands: prevented damage is not dealt, so the turn's
/// tally never moves, and Simulacrum, cast afterward, gains 0.
#[test]
fn simulacrum_gains_nothing_when_the_only_damage_this_turn_was_prevented() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), plains(), tyrranax_rex()])
        .hand(0, &[healing_salve(), simulacrum()])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, healing_salve());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected a mode choice, got {:?}", engine.pending())
    };
    let slot = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(1)))
        .expect("mode 1 is offered");
    engine.apply(p0, PlayerAction::ChooseMode(slot)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .expect("p0 is any target");
    pass_until(&mut engine, stack_is_empty);

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected p1 priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    aimed(&mut engine, p1, lightning_bolt(), &[], &[p0]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().per_turn.damage_dealt_to[0],
        0,
        "the shield absorbed all 3: nothing was dealt"
    );

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    let target = on_battlefield(&engine, p0, tyrranax_rex()).expect("Tyrranax Rex is seated");
    let life_before_the_spell = engine.state().players[0].life;
    cast_from_hand(&mut engine, p0, simulacrum());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        life_before_the_spell,
        "0: the only damage this turn never happened"
    );
}

/// Healing Salve's shield can sit on Simulacrum's *other* target instead of
/// on p0: the 3 damage this turn is genuine (an unshielded Bolt at p0
/// himself), so the life gain is unaffected, but the 3 Simulacrum would
/// mark its shielded creature with is prevented before any of it lands.
#[test]
fn simulacrum_s_damage_to_its_target_can_be_prevented_without_touching_the_life_gained() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), plains(), tyrranax_rex()])
        .hand(0, &[healing_salve(), simulacrum()])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected p1 priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    aimed(&mut engine, p1, lightning_bolt(), &[], &[p0]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().per_turn.damage_dealt_to[0],
        3,
        "the Bolt's 3, unshielded"
    );

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    let target = on_battlefield(&engine, p0, tyrranax_rex()).expect("Tyrranax Rex is seated");

    cast_from_hand(&mut engine, p0, healing_salve());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected a mode choice, got {:?}", engine.pending())
    };
    let slot = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(1)))
        .expect("mode 1 is offered");
    engine.apply(p0, PlayerAction::ChooseMode(slot)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .expect("a creature is any target too");
    pass_until(&mut engine, stack_is_empty);

    let life_before_the_spell = engine.state().players[0].life;
    cast_from_hand(&mut engine, p0, simulacrum());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        life_before_the_spell + 3,
        "the life gain reads the turn's tally, which the shield on the \
         creature never touched"
    );
    assert_eq!(
        engine.state().object(target).map(|o| o.damage),
        Some(0),
        "the shield on the creature absorbed all 3 before any could be marked"
    );
}
