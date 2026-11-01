//! `cards/creatures/mv_3/dwarven_vigilantes.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn dwarven_vigilantes() -> CardIndex {
    card_index("b49baf80-237c-41ee-8a22-2b6a68a313e7")
}

/// Turn 3, p0's main phase: the Vigilantes and the opponent's Grizzly Bears,
/// both on the battlefield since the start (no summoning sickness).
fn board() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[dwarven_vigilantes()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let dwarf = on_battlefield(&engine, p0, dwarven_vigilantes()).unwrap();
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    (engine, dwarf, bear)
}

/// Declares the Vigilantes as the only attacker, at p1.
#[track_caller]
fn attack(engine: &mut Engine<RegistryLookup>, dwarf: ObjectId) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            PlayerId::new(0),
            PlayerAction::DeclareAttackers {
                attackers: vec![(dwarf, Defender::Player(PlayerId::new(1)))],
            },
        )
        .unwrap();
}

/// Unblocked (the Bears is tapped): the target is chosen as the trigger goes on
/// the stack, "you may" is asked on resolution, and on yes the Vigilantes
/// deals damage equal to its power (2, which kills the Bears) and no combat
/// damage, so the opponent stays at 20.
#[test]
fn dwarven_vigilantes_unblocked_deals_power_damage_to_target_creature_and_no_combat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, dwarf, bear) = board();
    engine.state.set_tapped(bear, true);
    assert_eq!(pt(&engine, dwarf), (2, 2));
    attack(&mut engine, dwarf);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    reach_their_main_phase(&mut engine, p1);
    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
    assert_eq!(
        engine.state().players[1].life,
        20,
        "no combat damage assigned"
    );
}

/// "You may": declining leaves the Bears untouched and the Vigilantes deals its
/// combat damage to the player.
#[test]
fn dwarven_vigilantes_declined_deals_combat_damage_to_the_player_instead() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, dwarf, bear) = board();
    engine.state.set_tapped(bear, true);
    attack(&mut engine, dwarf);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    reach_their_main_phase(&mut engine, p1);
    assert!(
        on_battlefield(&engine, p1, grizzly_bears()).is_some(),
        "nothing was dealt to the Bears"
    );
    assert_eq!(engine.state().object(bear).unwrap().damage, 0);
    assert_eq!(engine.state().players[1].life, 18, "the 2 combat damage");
}

/// "Attacks and isn't blocked": when the Bears blocks, the trigger never
/// happens (`pass_until` refuses a target question) and the two trade.
#[test]
fn dwarven_vigilantes_blocked_does_not_trigger() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, dwarf, bear) = board();
    attack(&mut engine, dwarf);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bear, dwarf)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatEnd
    });
    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
    assert!(in_graveyard(&engine, p0, dwarven_vigilantes()).is_some());
    assert_eq!(engine.state().players[1].life, 20);
}
