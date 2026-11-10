//! `cards/creatures/mv_4/nantuko_vigilante.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::subtypes;

fn nantuko_vigilante() -> CardIndex {
    card_index("28ea78dc-7ad6-4006-aaf8-4b25057ec019")
}

#[track_caller]
fn cast_vigilante_face_down(engine: &mut Engine<RegistryLookup>, player: PlayerId) -> ObjectId {
    cast_from_hand(engine, player, nantuko_vigilante());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!(
            "normal and morph casts should be offered: {:?}",
            engine.pending()
        );
    };
    let morph = options
        .iter()
        .position(|option| matches!(option.kind, CastModeKind::Disguise))
        .expect("the shared face-down cast mode includes morph");
    engine
        .apply(player, PlayerAction::ChooseMode(morph))
        .expect("cast Nantuko Vigilante face down for {3}");
    pass_until(engine, stack_is_empty);

    let vigilante = on_battlefield(engine, player, nantuko_vigilante())
        .expect("the face-down creature resolved");
    let object = engine.state().object(vigilante).expect("the creature");
    assert!(object.status.contains(Status::FACE_DOWN));
    assert_eq!(pt(engine, vigilante), (2, 2));
    assert_eq!(types(engine, vigilante), TypeSet::CREATURE);
    vigilante
}

#[track_caller]
fn turn_vigilante_face_up(
    engine: &mut Engine<RegistryLookup>,
    player: PlayerId,
    vigilante: ObjectId,
) {
    engine
        .apply(
            player,
            PlayerAction::ActivateAbility {
                source: vigilante,
                ability_index: baylee_engine::choice::TURN_FACE_UP,
            },
        )
        .expect("the floating {1}{G} pays morph");
}

#[track_caller]
fn assert_face_up_vigilante(engine: &Engine<RegistryLookup>, vigilante: ObjectId) {
    let object = engine.state().object(vigilante).unwrap();
    assert!(!object.status.contains(Status::FACE_DOWN));
    assert_eq!(pt(engine, vigilante), (3, 2));
    assert!(types(engine, vigilante).contains(TypeSet::CREATURE));
    for subtype in [
        subtypes::creature::INSECT,
        subtypes::creature::DRUID,
        subtypes::creature::MUTANT,
    ] {
        assert!(object.characteristics().subtypes.contains(subtype));
    }
}

#[test]
fn nantuko_vigilante_turns_face_up_and_destroys_a_mandatory_artifact_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                quiet_artifact(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[nantuko_vigilante()])
        .battlefield(
            1,
            &[quiet_artifact(), their_enchantment(), llanowar_elves()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let vigilante = cast_vigilante_face_down(&mut engine, p0);
    let version = engine.state().object(vigilante).unwrap().version;
    let mana_before_morph = engine.state().players[0].mana_pool.total();
    let own_artifact = on_battlefield(&engine, p0, quiet_artifact()).unwrap();
    let own_creature = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let their_artifact = on_battlefield(&engine, p1, quiet_artifact()).unwrap();
    let their_enchantment = on_battlefield(&engine, p1, their_enchantment()).unwrap();
    let their_creature = on_battlefield(&engine, p1, llanowar_elves()).unwrap();

    turn_vigilante_face_up(&mut engine, p0, vigilante);

    assert_face_up_vigilante(&engine, vigilante);
    assert_eq!(
        engine.state().object(vigilante).unwrap().version,
        version,
        "turning face up changes no zone"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        mana_before_morph - 2,
        "the special action spends exactly {{1}}{{G}}"
    );
    assert!(
        stack_is_empty(&engine),
        "the special action does not use the stack; its trigger awaits targets"
    );

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the face-up trigger needs one target: {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1), "destruction is not optional");
    assert!(options.contains(&own_artifact), "own artifacts are legal");
    assert!(
        options.contains(&their_artifact),
        "their artifacts are legal"
    );
    assert!(
        options.contains(&their_enchantment),
        "enchantments are legal"
    );
    assert!(!options.contains(&own_creature));
    assert!(!options.contains(&their_creature));
    assert!(!options.contains(&vigilante));

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![their_artifact],
                players: vec![],
            },
        )
        .expect("target the opponent's artifact");
    assert_eq!(engine.state().zones.list(ZoneLocation::Stack).len(), 1);
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "choosing a target does not destroy it before the trigger resolves"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, quiet_artifact()).is_none());
    assert!(in_graveyard(&engine, p1, quiet_artifact()).is_some());
    assert!(on_battlefield(&engine, p0, quiet_artifact()).is_some());
    assert_eq!(
        engine.state().object(their_enchantment).unwrap().zone,
        Zone::Battlefield
    );
    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_some());
    assert!(on_battlefield(&engine, p1, llanowar_elves()).is_some());
}

#[test]
fn nantuko_vigilante_destroys_an_enchantment_when_turned_face_up() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 5])
        .hand(0, &[nantuko_vigilante()])
        .battlefield(1, &[their_enchantment()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let vigilante = cast_vigilante_face_down(&mut engine, p0);
    let enchantment = on_battlefield(&engine, p1, their_enchantment()).unwrap();

    turn_vigilante_face_up(&mut engine, p0, vigilante);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected the enchantment target: {:?}", engine.pending());
    };
    assert!(options.contains(&enchantment));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![enchantment],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, their_enchantment()).is_none());
    assert!(in_graveyard(&engine, p1, their_enchantment()).is_some());
}

#[test]
fn nantuko_vigilante_turns_face_up_without_a_target_prompt_when_none_is_legal() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 5])
        .hand(0, &[nantuko_vigilante()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let vigilante = cast_vigilante_face_down(&mut engine, p0);

    turn_vigilante_face_up(&mut engine, p0, vigilante);

    assert!(!matches!(engine.pending(), Pending::ChooseTargets { .. }));
    assert!(stack_is_empty(&engine));
    assert_eq!(pt(&engine, vigilante), (3, 2));
}

#[test]
fn nantuko_vigilante_does_not_destroy_a_replacement_when_its_target_leaves() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                quiet_artifact(),
            ],
        )
        .hand(0, &[nantuko_vigilante()])
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let vigilante = cast_vigilante_face_down(&mut engine, p0);
    let target = on_battlefield(&engine, p1, quiet_artifact()).unwrap();

    turn_vigilante_face_up(&mut engine, p0, vigilante);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
    bury(&mut engine, &[target]);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, quiet_artifact()).is_some());
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "an illegal original target does not retarget the trigger"
    );
}
