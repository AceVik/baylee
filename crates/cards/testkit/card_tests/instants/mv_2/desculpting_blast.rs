//! Desculpting Blast reads the target's attacking state before it left combat.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn desculpting_blast_creates_a_drone_after_returning_an_attacker() {
    play_desculpting_blast(BlastTarget::Attacker, 1);
}

#[test]
fn desculpting_blast_returns_another_nonland_without_creating_a_drone() {
    play_desculpting_blast(BlastTarget::Artifact, 0);
}

#[test]
fn desculpting_blast_returns_a_withdrawn_attacker_without_creating_a_drone() {
    play_desculpting_blast(BlastTarget::WithdrawnAttacker, 0);
}

#[test]
fn desculpting_blast_creates_a_drone_after_returning_a_face_down_attacker() {
    play_desculpting_blast(BlastTarget::FaceDownAttacker, 1);
}

#[derive(Clone, Copy)]
enum BlastTarget {
    Attacker,
    FaceDownAttacker,
    WithdrawnAttacker,
    Artifact,
}

fn play_desculpting_blast(blast_target: BlastTarget, expected_drones: usize) {
    let (attacking, defending) = (PlayerId::new(0), PlayerId::new(1));
    let blast = card_index("2d80ee9c-55d4-4115-8a58-63e8460f9795");
    let reconnaissance = card_index("32599d7c-243b-447b-9d5d-e8e29830ac61");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[grizzly_bears(), mox_opal(), reconnaissance, forest()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[blast])
        .start();
    keep_mulligans(&mut engine);
    let bears = on_battlefield(&engine, attacking, grizzly_bears()).unwrap();
    let opal = on_battlefield(&engine, attacking, mox_opal()).unwrap();
    if matches!(blast_target, BlastTarget::FaceDownAttacker) {
        let state = engine.dev_state_mut(attacking).unwrap();
        state
            .object_mut(bears)
            .unwrap()
            .status
            .insert(Status::FACE_DOWN);
        state.refresh_characteristics();
    }
    pass_until(
        &mut engine,
        |engine| matches!(engine.pending(), Pending::ChooseAttackers { player, .. } if *player == attacking),
    );
    let (target, returned) = if matches!(blast_target, BlastTarget::Artifact) {
        (opal, mox_opal())
    } else {
        (bears, grizzly_bears())
    };
    engine
        .apply(
            attacking,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(defending))],
            },
        )
        .unwrap();
    assert!(engine.state().combat.is_attacking(bears));
    if matches!(blast_target, BlastTarget::WithdrawnAttacker) {
        activate(&mut engine, attacking, reconnaissance, 0);
        aim_at(&mut engine, attacking, bears);
        pass_until(&mut engine, stack_is_empty);
        assert!(!engine.state().combat.is_attacking(bears));
    }
    pass_until(
        &mut engine,
        |engine| matches!(engine.pending(), Pending::Priority { player, .. } if *player == defending),
    );

    cast_from_hand(&mut engine, defending, blast);
    let targets = aim_at(&mut engine, defending, target);
    assert_eq!(targets.len(), 3);
    assert!(targets.contains(&bears) && targets.contains(&opal));
    if matches!(blast_target, BlastTarget::FaceDownAttacker) {
        assert!(
            engine
                .state()
                .object(bears)
                .unwrap()
                .status
                .contains(Status::FACE_DOWN)
        );
    }
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(in_hand(&engine, attacking, returned), Some(target));

    let drones = tokens_of(&engine, defending);
    assert_eq!(drones.len(), expected_drones);
    for drone in drones {
        let characteristics = engine.state().object(drone).unwrap().characteristics();
        assert!(characteristics.colors.is_empty());
        assert!(types(&engine, drone).contains(TypeSet::ARTIFACT.union(TypeSet::CREATURE)));
        assert!(
            characteristics
                .subtypes
                .contains(baylee_core::generated::subtypes::creature::DRONE)
        );
        assert_eq!(pt(&engine, drone), (1, 1));
        assert!(keywords(&engine, drone).contains(KeywordSet::FLYING));
    }
}
