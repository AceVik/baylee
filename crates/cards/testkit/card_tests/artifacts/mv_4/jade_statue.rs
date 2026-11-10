//! `cards/artifacts/mv_4/jade_statue.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Jade Statue: "{2}: This artifact becomes a 3/6 Golem artifact creature
/// until end of combat. Activate only during combat." Withheld outright
/// during the first main phase; offered once combat begins.
#[test]
fn jade_statue_animates_only_during_combat() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[jade_statue(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let statue = on_battlefield(&engine, p0, jade_statue()).expect("seated");
    assert!(
        !priority_offer(&engine).abilities.contains(&(statue, 0)),
        "not combat yet"
    );
    let types_before = engine
        .state()
        .object(statue)
        .unwrap()
        .characteristics()
        .types;
    assert!(
        !types_before.contains(TypeSet::CREATURE),
        "an inert artifact outside combat"
    );

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Combat)
    });
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, jade_statue(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, statue), (3, 6), "the printed 3/6");
    let types_after = engine
        .state()
        .object(statue)
        .unwrap()
        .characteristics()
        .types;
    assert!(types_after.contains(TypeSet::CREATURE) && types_after.contains(TypeSet::ARTIFACT));

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    let types_second_main = engine
        .state()
        .object(statue)
        .unwrap()
        .characteristics()
        .types;
    assert!(
        !types_second_main.contains(TypeSet::CREATURE),
        "\"until end of combat\" — no longer a creature once combat has ended"
    );
    assert_eq!(
        pt(&engine, statue),
        (0, 0),
        "a noncreature artifact carries no power or toughness"
    );
}

/// The animation's other three words: "Golem". In combat the Statue is a
/// Golem and no other creature type, and it is an artifact creature; before
/// and after combat it has no creature type at all.
#[test]
fn jade_statue_is_a_golem_artifact_creature_for_the_combat_only() {
    let p0 = PlayerId::new(0);
    let golem = baylee_core::generated::subtypes::creature::GOLEM;
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[jade_statue(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let statue = on_battlefield(&engine, p0, jade_statue()).expect("seated");
    let subtypes = |engine: &Engine<RegistryLookup>| {
        engine
            .state()
            .object(statue)
            .unwrap()
            .characteristics()
            .subtypes
    };
    assert_eq!(subtypes(&engine).iter().count(), 0, "no subtype at home");

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Combat)
    });
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, jade_statue(), 0);
    pass_until(&mut engine, stack_is_empty);
    let animated = subtypes(&engine);
    assert!(animated.contains(golem), "a Golem");
    assert_eq!(animated.iter().count(), 1, "and nothing but a Golem");
    let types = engine
        .state()
        .object(statue)
        .unwrap()
        .characteristics()
        .types;
    assert!(
        types.contains(TypeSet::ARTIFACT) && types.contains(TypeSet::CREATURE),
        "an artifact creature"
    );

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(
        subtypes(&engine).iter().count(),
        0,
        "until end of combat: no longer a Golem"
    );
}
