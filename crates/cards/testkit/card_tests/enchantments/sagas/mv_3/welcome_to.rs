//! `cards/enchantments/sagas/mv_3/welcome_to.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Natural chapter progression creates the Dinosaur, transforms the Saga and
/// counts both the original creature and the new token for Jurassic Park mana.
#[test]
fn welcome_to_advances_to_chapter_three_and_taps_for_dinosaur_mana() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(102, forest())
        .battlefield(0, &[forest(), forest(), forest(), carnage_tyrant()])
        .hand(0, &[welcome_to()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, welcome_to());
    pass_until(&mut engine, stack_is_empty);

    let saga = on_battlefield(&engine, p0, welcome_to()).expect("Welcome to . . . on battlefield");
    assert_eq!(
        counters_on(&engine, saga, CounterKind::Lore),
        1,
        "enters with one lore counter"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let saga = on_battlefield(&engine, p0, welcome_to())
        .expect("Welcome to . . . on battlefield in turn 2");
    assert_eq!(
        counters_on(&engine, saga, CounterKind::Lore),
        2,
        "second lore counter added in precombat main phase"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    // Chapter III triggered at the start of turn 3's precombat main phase; resolve it.
    pass_until(&mut engine, stack_is_empty);

    let jp = on_battlefield(&engine, p0, welcome_to()).expect("Jurassic Park on battlefield");
    assert_eq!(
        engine.state().object(jp).expect("object exists").face_index,
        1,
        "transformed to face 1"
    );
    assert!(
        types(&engine, jp).contains(TypeSet::LAND),
        "Jurassic Park is a land"
    );

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        0,
        "mana pool is empty before activating Jurassic Park"
    );

    activate(&mut engine, p0, welcome_to(), 1);

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        2,
        "the original Dinosaur and chapter II token each produce green mana"
    );
}

/// Casts Welcome to . . . with an opponent's Sol Ring on the table, aims
/// chapter I at it and lets it resolve.
fn welcome_aimed_at_a_sol_ring() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    use baylee_core::generated::subtypes::creature::WALL;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(102, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[welcome_to()])
        .battlefield(1, &[sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ring = on_battlefield(&engine, p1, sol_ring()).expect("the opponent's Sol Ring");
    assert!(
        !types(&engine, ring).contains(TypeSet::CREATURE),
        "the Sol Ring is no creature before chapter I"
    );

    cast_from_hand(&mut engine, p0, welcome_to());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("the walk stops on the target choice")
    };
    assert_eq!(
        options,
        vec![ring],
        "\"noncreature artifact they control\": the Sol Ring is the one candidate"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ring],
                players: vec![],
            },
        )
        .expect("the Sol Ring is a legal target");
    pass_until(&mut engine, stack_is_empty);
    let saga = on_battlefield(&engine, p0, welcome_to()).expect("the Saga");
    assert!(
        engine
            .state()
            .object(ring)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(WALL),
        "chapter I resolved"
    );
    (engine, saga, ring)
}

/// "becomes a 0/4 Wall artifact creature with defender": every part of the
/// sentence, the type line included, not only the 0/4 and the defender.
#[test]
fn welcome_chapter_one_makes_the_target_a_wall_artifact_creature_with_defender() {
    let (engine, _saga, ring) = welcome_aimed_at_a_sol_ring();
    let t = types(&engine, ring);
    assert!(t.contains(TypeSet::CREATURE), "it becomes a creature");
    assert!(t.contains(TypeSet::ARTIFACT), "and stays an artifact");
    assert!(
        engine
            .state()
            .object(ring)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::WALL),
        "it is a Wall"
    );
    assert_eq!(pt(&engine, ring), (0, 4), "0/4");
    assert!(
        keywords(&engine, ring).contains(KeywordSet::DEFENDER),
        "with defender"
    );
}

/// "for as long as you control this Saga": once the Saga is gone the Sol Ring
/// is the plain artifact it was, with no Wall, no 0/4 and no defender.
#[test]
fn welcome_chapter_one_lasts_only_while_its_controller_controls_the_saga() {
    let (mut engine, saga, ring) = welcome_aimed_at_a_sol_ring();
    kill(&mut engine, saga);
    assert_ne!(
        engine.state().object(saga).unwrap().zone,
        Zone::Battlefield,
        "the Saga left the battlefield"
    );

    let t = types(&engine, ring);
    assert!(
        !t.contains(TypeSet::CREATURE),
        "the Sol Ring is no creature once the Saga is gone"
    );
    assert!(t.contains(TypeSet::ARTIFACT), "it is still an artifact");
    assert!(
        !engine
            .state()
            .object(ring)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::WALL),
        "it is no Wall any more"
    );
    assert!(
        !keywords(&engine, ring).contains(KeywordSet::DEFENDER),
        "and has no defender"
    );
}

/// "It gains haste until end of turn": the Dinosaur token has haste on the
/// turn chapter II made it and trample after, haste gone.
#[test]
fn welcome_chapter_two_dinosaur_has_haste_only_until_end_of_turn_and_keeps_trample() {
    use baylee_core::generated::subtypes::creature::DINOSAUR;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(102, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[welcome_to()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, welcome_to());
    pass_until(&mut engine, stack_is_empty);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let is_dinosaur = |e: &Engine<RegistryLookup>, id: ObjectId| {
        e.state()
            .object(id)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(DINOSAUR)
    };
    pass_until(&mut engine, |e| {
        e.state().battlefield_seen().any(|id| is_dinosaur(e, id))
    });
    let dinosaur = engine
        .state()
        .battlefield_seen()
        .find(|id| is_dinosaur(&engine, *id))
        .expect("chapter II's Dinosaur token");
    assert!(
        keywords(&engine, dinosaur).contains(KeywordSet::HASTE),
        "on the turn it was made it has haste"
    );
    assert!(keywords(&engine, dinosaur).contains(KeywordSet::TRAMPLE));

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().object(dinosaur).unwrap().zone,
        Zone::Battlefield,
        "the token is still there on the next turn"
    );
    assert!(
        !keywords(&engine, dinosaur).contains(KeywordSet::HASTE),
        "\"until end of turn\": haste is gone on the next turn"
    );
    assert!(
        keywords(&engine, dinosaur).contains(KeywordSet::TRAMPLE),
        "the token's own trample is not part of the temporary grant"
    );
}

/// "{T}: Add {G} for each Dinosaur you control": a Dinosaur the opponent
/// controls adds nothing to it.
#[test]
fn jurassic_park_counts_only_the_dinosaurs_its_controller_controls() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(102, forest())
        .battlefield(0, &[forest(), forest(), forest(), carnage_tyrant()])
        .battlefield(1, &[carnage_tyrant()])
        .hand(0, &[welcome_to()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, welcome_to());
    pass_until(&mut engine, stack_is_empty);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    pass_until(&mut engine, stack_is_empty);

    let park = on_battlefield(&engine, p0, welcome_to()).expect("Jurassic Park");
    assert_eq!(engine.state().object(park).unwrap().face_index, 1);
    assert!(
        on_battlefield(&engine, p1, carnage_tyrant()).is_some(),
        "the opponent's Dinosaur is on the battlefield"
    );
    let green_before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Green);

    activate(&mut engine, p0, welcome_to(), 1);

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green)
            - green_before,
        2,
        "its controller's Carnage Tyrant and chapter II's token count; the opponent's Tyrant does not"
    );
}
