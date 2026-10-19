//! `cards/artifacts/mv_3/spirit_shield.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn spirit_shield() -> CardIndex {
    card_index("db53fad4-03b8-47bb-9459-b1eb6cca961c")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// What the pump adds to a Grizzly Bears (2/2) or a Hill Giant (3/3).
const BEARS: ((i16, i16), (i16, i16)) = ((2, 2), (2, 4));
const GIANT: ((i16, i16), (i16, i16)) = ((3, 3), (3, 5));

/// Turn one, main phase: the artifact and its Forests, Bears on my side, a
/// Hill Giant across the table. The artifact is activated at `theirs` (or
/// my Bears) and resolved; the engine, the artifact and the target return.
fn pumped(at_theirs: bool) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut mine = vec![spirit_shield(), grizzly_bears()];
    mine.extend([forest(); 2]);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &mine)
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let artifact = on_battlefield(&engine, p0, spirit_shield()).expect("the artifact");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("my Bears");
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("their Giant");
    assert_eq!(pt(&engine, bears), BEARS.0);
    assert_eq!(pt(&engine, giant), GIANT.0);

    assert_eq!(tap_all_mana(&mut engine, p0), 2, "every Forest pays");
    activate(&mut engine, p0, spirit_shield(), 1);
    let target = if at_theirs { giant } else { bears };
    let options = aim_at(&mut engine, p0, target);
    assert!(
        options.contains(&bears) && options.contains(&giant),
        "any creature, the opponent's too: {options:?}"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, artifact), "{{T}} was part of the price");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and so was the mana"
    );
    (engine, artifact, target)
}

fn expected(engine: &Engine<RegistryLookup>, target: ObjectId) -> ((i16, i16), (i16, i16)) {
    if engine.state().object(target).unwrap().controller == PlayerId::new(0) {
        BEARS
    } else {
        GIANT
    }
}

/// Walks until the untap step asks which permanents stay tapped, handing the
/// question back unanswered (`answer_one` would untap).
#[track_caller]
fn to_the_untap_question(engine: &mut Engine<RegistryLookup>) -> Vec<ObjectId> {
    for _ in 0..200 {
        if let Pending::ChooseCards {
            options,
            prompt: ChoicePrompt::LeaveTapped,
            ..
        } = engine.pending().clone()
        {
            return options;
        }
        let (player, action) = answer_one(engine).expect("a rest on the way");
        engine.apply(player, action).expect("legal");
    }
    panic!("the untap step never asked");
}

/// x: on either side's creature.
#[test]
fn spirit_shield_pumps_any_creature_while_tapped() {
    let (engine, _artifact, target) = pumped(true);
    assert_eq!(pt(&engine, target), expected(&engine, target).1);
    let (engine, _artifact, target) = pumped(false);
    assert_eq!(pt(&engine, target), expected(&engine, target).1);
}

/// "You may choose not to untap this artifact during your untap step": kept
/// tapped through p0's next untap step, the pump goes on across the turns.
#[test]
fn spirit_shield_lasts_while_it_is_kept_tapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, artifact, target) = pumped(false);
    reach_their_main_phase(&mut engine, p1);
    let options = to_the_untap_question(&mut engine);
    assert!(options.contains(&artifact), "the artifact is on the menu");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![artifact],
            },
        )
        .expect("leave it tapped");
    reach_main_phase(&mut engine, p0);
    assert!(is_tapped(&engine, artifact), "it stayed down");
    assert_eq!(
        pt(&engine, target),
        expected(&engine, target).1,
        "\"for as long as this artifact remains tapped\""
    );
}

/// Untapping it normally ends the pump.
#[test]
fn spirit_shield_pump_ends_when_it_untaps() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, artifact, target) = pumped(false);
    reach_their_main_phase(&mut engine, p1);
    to_the_untap_question(&mut engine);
    assert_eq!(
        pt(&engine, target),
        expected(&engine, target).1,
        "still tapped at the question"
    );
    let (player, action) = answer_one(&engine).expect("the question");
    engine.apply(player, action).expect("legal");
    reach_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, artifact));
    assert_eq!(pt(&engine, target), expected(&engine, target).0);
}
