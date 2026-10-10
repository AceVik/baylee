//! `cards/creatures/mv_5/argothian_treefolk.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "f3aaef18-dc32-40d6-b48c-f957aa31247f"
fn argothian_treefolk() -> CardIndex {
    card_index("f3aaef18-dc32-40d6-b48c-f957aa31247f")
}

/// Damage marked on `id` right now (cleanup wipes it, so every read here is
/// taken before the turn ends).
fn marked(engine: &Engine<RegistryLookup>, id: ObjectId) -> u16 {
    engine.state().object(id).expect("on the table").damage
}

/// A 3/5 for {3}{G}{G}.
#[test]
fn argothian_treefolk_is_the_three_five_it_prints() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[argothian_treefolk()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let treefolk = on_battlefield(&engine, p0, argothian_treefolk()).expect("seated");
    assert_eq!(pt(&engine, treefolk), (3, 5));
}

/// "Prevent all damage that would be dealt to this creature by artifact
/// sources." Rod of Ruin's ping is damage from an artifact source, so the
/// Treefolk takes none of it -- yet the Rod may still name the Treefolk: this
/// is prevention, not protection (CR 615 against CR 702.16). Prodigal
/// Sorcerer, a nonartifact source, damages the same creature for the control.
#[test]
fn argothian_treefolk_takes_no_damage_from_an_artifact_ability_but_still_can_be_targeted() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                argothian_treefolk(),
                rod_of_ruin(),
                prodigal_sorcerer(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let treefolk = on_battlefield(&engine, p0, argothian_treefolk()).expect("seated");

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, rod_of_ruin(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Rod of Ruin aims, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&treefolk),
        "an artifact ability may target the Treefolk: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![treefolk],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        marked(&engine, treefolk),
        0,
        "the Rod's damage is prevented"
    );
    assert!(
        on_battlefield(&engine, p0, argothian_treefolk()).is_some(),
        "and it is still there"
    );

    activate(&mut engine, p0, prodigal_sorcerer(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![treefolk],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        marked(&engine, treefolk),
        1,
        "a nonartifact creature's ability still hurts it"
    );
}

/// A nonartifact spell is not prevented: Lightning Bolt marks three.
#[test]
fn argothian_treefolk_takes_damage_from_a_nonartifact_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[argothian_treefolk(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let treefolk = on_battlefield(&engine, p0, argothian_treefolk()).expect("seated");

    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![treefolk],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(marked(&engine, treefolk), 3);
}

/// Attacks with the Treefolk into one blocker and returns the engine after
/// combat damage, still in the turn.
fn treefolk_blocked_by(blocker_card: CardIndex) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[argothian_treefolk()])
        .battlefield(1, &[blocker_card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let treefolk = on_battlefield(&engine, p0, argothian_treefolk()).expect("seated");
    let blocker = on_battlefield(&engine, p1, blocker_card).expect("seated");
    let offered = attack_and_collect_blocks(&mut engine, treefolk, p1);
    assert!(
        offered.iter().any(|b| b.blocker == blocker),
        "prevention is no evasion: the blocker may block"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, treefolk)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    (engine, treefolk, blocker)
}

/// An artifact creature blocks and is dealt 3, but the 4 it deals back is
/// prevented.
#[test]
fn argothian_treefolk_takes_no_combat_damage_from_an_artifact_creature() {
    let (engine, treefolk, su_chi_id) = treefolk_blocked_by(su_chi());
    assert_eq!(marked(&engine, treefolk), 0, "Su-Chi is an artifact source");
    assert_eq!(
        marked(&engine, su_chi_id),
        3,
        "the Treefolk's own damage lands"
    );
}

/// A nonartifact creature in the same spot does hurt it.
#[test]
fn argothian_treefolk_takes_combat_damage_from_a_nonartifact_creature() {
    let (engine, treefolk, bears) = treefolk_blocked_by(grizzly_bears());
    assert_eq!(marked(&engine, treefolk), 2);
    assert!(
        in_graveyard(&engine, PlayerId::new(1), grizzly_bears()).is_some(),
        "the Bears took 3 and died ({bears:?})"
    );
}
