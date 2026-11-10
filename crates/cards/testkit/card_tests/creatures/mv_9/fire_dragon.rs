//! `cards/creatures/mv_9/fire_dragon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn fire_dragon() -> CardIndex {
    card_index("349d8ef9-e07a-416f-a5b1-2c2be6bb322d")
}

fn scaled_wurm() -> CardIndex {
    card_index("009a8e38-74d0-4b4a-b8ca-62c9f3183531")
}

/// Fire Dragon is a 6/6 flier for {6}{R}{R}{R}. Its enter trigger deals
/// damage equal to the Mountains its controller has to a target creature: it
/// is cast on `mountains` Mountains and `9 - mountains` Forests against the
/// opponent's 7/6 Scaled Wurm, and the Wurm's marked damage is returned.
fn cast_against_a_wurm(mountains: usize) -> (Engine<RegistryLookup>, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut lands = vec![mountain(); mountains];
    lands.extend(vec![forest(); 9 - mountains]);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &lands)
        .battlefield(1, &[scaled_wurm()])
        .hand(0, &[fire_dragon()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));
    let wurm = on_battlefield(&engine, p1, scaled_wurm()).expect("their Wurm");
    cast_from_hand(&mut engine, p0, fire_dragon());
    let options = pass_until_targets(&mut engine, p0);
    assert!(options.contains(&wurm), "{options:?}");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![wurm],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    (engine, wurm)
}

/// "When this creature enters, it deals damage to target creature equal to
/// the number of Mountains you control": six Mountains deal 6, which kills a
/// toughness-6 Wurm.
#[test]
fn fire_dragon_with_six_mountains_kills_a_toughness_six_creature() {
    let (engine, _) = cast_against_a_wurm(6);
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    assert!(on_battlefield(&engine, p0, fire_dragon()).is_some());
    assert!(
        on_battlefield(&engine, p1, scaled_wurm()).is_none(),
        "6 damage"
    );
}

/// Five Mountains deal only 5: the Wurm survives, marked for 5. Forests do
/// not count.
#[test]
fn fire_dragon_counts_only_mountains() {
    let (engine, wurm) = cast_against_a_wurm(5);
    assert_eq!(engine.state().object(wurm).expect("alive").damage, 5);
    assert_eq!(pt(&engine, wurm), (7, 6));
}

/// The Dragon is a flier with the printed body.
#[test]
fn fire_dragon_is_a_six_six_flier() {
    let (engine, _) = cast_against_a_wurm(9);
    let dragon = on_battlefield(&engine, PlayerId::new(0), fire_dragon()).expect("the Dragon");
    assert_eq!(pt(&engine, dragon), (6, 6));
    assert!(keywords(&engine, dragon).contains(KeywordSet::FLYING));
}
