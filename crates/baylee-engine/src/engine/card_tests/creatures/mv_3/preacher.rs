//! `cards/creatures/mv_3/preacher.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::ChoicePrompt;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn preacher() -> CardIndex {
    card_index("d5ea9176-30c1-4152-9086-322f39245f46")
}

fn controller(engine: &Engine<RegistryLookup>, id: ObjectId) -> PlayerId {
    engine.state().object(id).expect("the object").controller
}

/// Turn one, main phase: the Preacher activates against an opponent holding a
/// Gray Ogre and a Llanowar Elves; the opponent picks the Ogre to give up.
fn preacher_has_the_ogre() -> (Engine<RegistryLookup>, ObjectId, ObjectId, ObjectId) {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[preacher()])
        .battlefield(1, &[gray_ogre(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let priest = on_battlefield(&engine, P0, preacher()).expect("the Preacher is out");
    let ogre = on_battlefield(&engine, P1, gray_ogre()).expect("their Ogre");
    let elf = on_battlefield(&engine, P1, quiet_creature()).expect("their Elf");

    activate(&mut engine, P0, preacher(), 1);
    // "of an opponent's choice": the opponent is the one asked.
    let menu = aim_at(&mut engine, P1, ogre);
    assert!(menu.contains(&ogre) && menu.contains(&elf), "{menu:?}");
    assert!(
        !menu.contains(&priest),
        "only creatures that opponent controls"
    );
    pass_until(&mut engine, stack_is_empty);
    (engine, priest, ogre, elf)
}

/// Walks until the untap step asks which permanents stay tapped, handing the
/// question back unanswered.
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

/// Preacher (d5ea9176-30c1-4152-9086-322f39245f46) — "{T}: For as long as
/// this creature remains tapped, gain control of target creature of an
/// opponent's choice they control." The opponent chooses (the Ogre, not the
/// Elf); I control it, and keep it through their turn while the Preacher is
/// tapped.
#[test]
fn preacher_gains_control_of_the_creature_the_opponent_chooses() {
    let (engine, priest, ogre, elf) = preacher_has_the_ogre();
    assert!(is_tapped(&engine, priest), "{{T}} was paid");
    assert_eq!(controller(&engine, ogre), P0, "the Ogre is mine now");
    assert_eq!(controller(&engine, elf), P1, "the Elf was not chosen");
}

#[test]
fn preacher_keeps_the_creature_through_their_turn() {
    let (mut engine, priest, ogre, _elf) = preacher_has_the_ogre();
    reach_their_main_phase(&mut engine, P1);
    assert!(is_tapped(&engine, priest));
    assert_eq!(controller(&engine, ogre), P0, "still tapped, still mine");
}

/// The Preacher untapping (my untap step) ends the control effect.
#[test]
fn preacher_untapping_gives_the_creature_back() {
    let (mut engine, priest, ogre, _elf) = preacher_has_the_ogre();
    reach_their_main_phase(&mut engine, P1);
    let options = to_the_untap_question(&mut engine);
    assert!(options.contains(&priest), "I may leave the Preacher tapped");
    let (player, action) = answer_one(&engine).expect("the question");
    engine.apply(player, action).expect("legal");
    reach_main_phase(&mut engine, P0);
    assert!(!is_tapped(&engine, priest), "untapped");
    assert_eq!(controller(&engine, ogre), P1, "it went home");
}

/// "You may choose not to untap this creature": leaving it tapped keeps the
/// Ogre into my next turn.
#[test]
fn preacher_left_tapped_keeps_the_creature() {
    let (mut engine, priest, ogre, _elf) = preacher_has_the_ogre();
    reach_their_main_phase(&mut engine, P1);
    to_the_untap_question(&mut engine);
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![priest],
            },
        )
        .expect("leave it tapped");
    reach_main_phase(&mut engine, P0);
    assert!(is_tapped(&engine, priest), "it stayed tapped");
    assert_eq!(controller(&engine, ogre), P0, "so the Ogre is still mine");
}

/// The Preacher leaving the battlefield ends the effect too.
#[test]
fn preacher_dying_gives_the_creature_back() {
    let (mut engine, priest, ogre, _elf) = preacher_has_the_ogre();
    kill(&mut engine, priest);
    assert!(in_graveyard(&engine, P0, preacher()).is_some());
    assert_eq!(controller(&engine, ogre), P1);
}
