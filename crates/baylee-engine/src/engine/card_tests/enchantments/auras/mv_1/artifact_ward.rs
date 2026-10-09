//! `cards/enchantments/auras/mv_1/artifact_ward.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "9bd3a4bb-cc12-4e5f-a33f-77ab0c7788db"
fn artifact_ward() -> CardIndex {
    card_index("9bd3a4bb-cc12-4e5f-a33f-77ab0c7788db")
}

// oracle_id = "5e961d15-5972-4e4b-9385-1cd7cd7c6bbe"
fn prodigal_sorcerer() -> CardIndex {
    card_index("5e961d15-5972-4e4b-9385-1cd7cd7c6bbe")
}

// oracle_id = "9a28d53e-c789-47de-8f3e-a251843ac596"
fn su_chi() -> CardIndex {
    card_index("9a28d53e-c789-47de-8f3e-a251843ac596")
}

// oracle_id = "5ccb57e1-ca94-4b5a-8e5f-b8b5e692cfb9"
fn steel_wall() -> CardIndex {
    card_index("5ccb57e1-ca94-4b5a-8e5f-b8b5e692cfb9")
}

/// Damage marked on `id` right now (cleanup wipes it).
fn marked(engine: &Engine<RegistryLookup>, id: ObjectId) -> u16 {
    engine.state().object(id).expect("on the table").damage
}

/// Seat 0 holds Grizzly Bears, Artifact Ward in hand and the given extras
/// (four Plains besides); Ward is cast on the Bears and resolved with three
/// mana still floating. Returns the engine and the Bears.
fn warded_bears(ours: &[CardIndex], theirs: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut board = vec![grizzly_bears(), plains(), plains(), plains(), plains()];
    board.extend_from_slice(ours);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &board)
        .battlefield(1, theirs)
        .hand(0, &[artifact_ward()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, artifact_ward());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Enchant creature asks, got {:?}", engine.pending())
    };
    assert!(options.contains(&bears));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let aura = on_battlefield(&engine, p0, artifact_ward()).expect("resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(bears)
    );
    (engine, bears)
}

/// "Enchanted creature can't be blocked by artifact creatures."
#[test]
fn artifact_ward_stops_artifact_creatures_from_blocking_but_not_others() {
    let p1 = PlayerId::new(1);
    let (mut engine, bears) = warded_bears(&[], &[su_chi(), steel_wall(), grizzly_bears()]);
    let su_chi_id = on_battlefield(&engine, p1, su_chi()).expect("seated");
    let wall = on_battlefield(&engine, p1, steel_wall()).expect("seated");
    let their_bears = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");

    let offered = attack_and_collect_blocks(&mut engine, bears, p1);
    let can_block = |id: ObjectId| {
        offered
            .iter()
            .any(|b| b.blocker == id && b.attackers.contains(&bears))
    };
    assert!(can_block(their_bears), "a nonartifact creature may block");
    assert!(!can_block(su_chi_id));
    assert!(!can_block(wall));
}

/// "Prevent all damage that would be dealt to enchanted creature by artifact
/// sources": Su-Chi attacks and the warded Bears block, taking nothing.
#[test]
fn artifact_ward_prevents_combat_damage_from_an_artifact_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, bears) = warded_bears(&[], &[su_chi()]);
    let attacker = on_battlefield(&engine, p1, su_chi()).expect("seated");
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, attacker)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(marked(&engine, bears), 0, "Su-Chi's 4 is prevented");
    assert_eq!(marked(&engine, attacker), 2, "the Bears hit back");
}

/// "Enchanted creature can't be the target of abilities from artifact
/// sources." Rod of Ruin may not name the Bears, though it may name a player;
/// a nonartifact source (Prodigal Sorcerer) still may, and hurts it.
#[test]
fn artifact_ward_hides_the_creature_from_artifact_abilities_only() {
    let p0 = PlayerId::new(0);
    let (mut engine, bears) = warded_bears(&[rod_of_ruin(), prodigal_sorcerer()], &[]);

    activate(&mut engine, p0, rod_of_ruin(), 0);
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("Rod of Ruin aims, got {:?}", engine.pending())
    };
    assert!(
        !options.contains(&bears),
        "an ability of an artifact can't target the warded creature: {options:?}"
    );
    assert!(player_options.contains(&p0), "players are still legal");
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![bears]
                }
            )
            .is_err(),
        "and the engine refuses it if asked"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![PlayerId::new(1)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(marked(&engine, bears), 0);

    activate(&mut engine, p0, prodigal_sorcerer(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Prodigal Sorcerer aims, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&bears),
        "a nonartifact ability may target it: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(marked(&engine, bears), 1, "and its damage is not prevented");
}
