//! `cards/artifacts/equipment/mv_1/spare_dagger.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn spare_dagger() -> CardIndex {
    card_index("d895df97-e7e7-4f15-a79b-0ce8f1a252d3")
}

fn walking_corpse() -> CardIndex {
    card_index("fea95888-e16a-4209-9cd4-623f7f4d2f67")
}

fn declare_attack(engine: &mut Engine<RegistryLookup>, attacker: ObjectId) {
    let p0 = PlayerId::new(0);
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(PlayerId::new(1)))],
            },
        )
        .unwrap();
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: baylee_engine::choice::YesNoPrompt::MayDo,
                ..
            }
        )
    });
}

/// The printed +1/+0 follows the Equipment rather than either creature.
#[test]
fn dagger_moves_its_power_bonus() {
    plain_pump(spare_dagger(), 1, 1, 0);
}

/// Declining leaves the Dagger attached and creates neither a target question
/// nor a damage event. The ordinary attack still proceeds normally.
#[test]
fn dagger_may_be_kept_without_a_reflexive_trigger() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = rig(1, &[], &[spare_dagger(), tragic_poet()], &[]);
    let attacker = on_battlefield(&engine, p0, tragic_poet()).unwrap();
    let dagger = equip(&mut engine, spare_dagger(), 1, attacker);
    declare_attack(&mut engine, attacker);
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert_eq!(
        engine.state().object(dagger).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        engine.state().object(dagger).unwrap().attached_to,
        Some(attacker)
    );
    assert_eq!(engine.state().players[1].life, 20);
    assert!(
        !engine
            .state()
            .journal
            .entries()
            .iter()
            .any(|entry| matches!(
                entry.event,
                GameEvent::DamageDealt {
                    is_combat: false,
                    ..
                }
            ))
    );
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().players[1].life,
        18,
        "only the 2/1 attacker hit"
    );
}

/// Accepting sacrifices the Dagger during the attack trigger's resolution.
/// Only then does a separate reflexive trigger ask for any target; the
/// equipped creature, not the Dagger, deals its one damage.
#[test]
fn dagger_sacrifice_creates_a_separate_targeted_trigger_from_the_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = rig(
        1,
        &[],
        &[spare_dagger(), tragic_poet()],
        &[walking_corpse()],
    );
    let attacker = on_battlefield(&engine, p0, tragic_poet()).unwrap();
    let dagger = equip(&mut engine, spare_dagger(), 1, attacker);
    let corpse = on_battlefield(&engine, p1, walking_corpse()).unwrap();
    declare_attack(&mut engine, attacker);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();

    assert_eq!(engine.state().object(dagger).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        engine.state().object(attacker).unwrap().zone,
        Zone::Battlefield
    );
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the reflexive trigger asks for any target: {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&corpse), "a creature is any target");
    assert!(player_options.contains(&p1), "a player is any target");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    assert_eq!(engine.state().players[1].life, 19);
    assert!(
        engine
            .state()
            .journal
            .entries()
            .iter()
            .any(|entry| matches!(
                entry.event,
                GameEvent::DamageDealt {
                    source: Some(source),
                    target: baylee_engine::event::DamageTarget::Player(player),
                    amount: 1,
                    is_combat: false,
                } if source == attacker && player == p1
            )),
        "the equipped creature, not the sacrificed Dagger, dealt the one"
    );
}
