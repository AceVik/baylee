//! `cards/creatures/mv_3/seasinger.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::ChoicePrompt;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn seasinger() -> CardIndex {
    card_index("13745b92-1e30-4075-9733-5d00da11408c")
}

fn controller(engine: &Engine<RegistryLookup>, id: ObjectId) -> PlayerId {
    engine.state().object(id).expect("the object").controller
}

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

/// Turn one, main phase: both sides have an Island; mine has the Seasinger,
/// theirs a Gray Ogre. The ability (index 2) is activated at the Ogre.
fn seasinger_has_the_ogre() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[seasinger(), island()])
        .battlefield(1, &[gray_ogre(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let sea = on_battlefield(&engine, P0, seasinger()).expect("the Seasinger is out");
    let ogre = on_battlefield(&engine, P1, gray_ogre()).expect("their Ogre");
    activate(&mut engine, P0, seasinger(), 2);
    aim_at(&mut engine, P0, ogre);
    pass_until(&mut engine, stack_is_empty);
    (engine, sea, ogre)
}

/// Seasinger (13745b92-1e30-4075-9733-5d00da11408c) — "{T}: Gain control of
/// target creature whose controller controls an Island for as long as you
/// control this creature and this creature remains tapped."
#[test]
fn seasinger_takes_a_creature_whose_controller_has_an_island() {
    let (engine, sea, ogre) = seasinger_has_the_ogre();
    assert!(is_tapped(&engine, sea), "{{T}} was paid");
    assert_eq!(controller(&engine, ogre), P0);
}

/// It ends when the Seasinger untaps.
#[test]
fn seasinger_untapping_returns_the_creature() {
    let (mut engine, sea, ogre) = seasinger_has_the_ogre();
    reach_their_main_phase(&mut engine, P1);
    assert_eq!(controller(&engine, ogre), P0, "tapped through their turn");
    let options = to_the_untap_question(&mut engine);
    assert!(options.contains(&sea), "I may choose not to untap it");
    let (player, action) = answer_one(&engine).expect("the question");
    engine.apply(player, action).expect("legal");
    reach_main_phase(&mut engine, P0);
    assert!(!is_tapped(&engine, sea));
    assert_eq!(controller(&engine, ogre), P1, "it went home");
}

/// "You may choose not to untap this creature during your untap step."
#[test]
fn seasinger_left_tapped_keeps_the_creature() {
    let (mut engine, sea, ogre) = seasinger_has_the_ogre();
    reach_their_main_phase(&mut engine, P1);
    to_the_untap_question(&mut engine);
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![sea] })
        .expect("leave it tapped");
    reach_main_phase(&mut engine, P0);
    assert!(is_tapped(&engine, sea));
    assert_eq!(controller(&engine, ogre), P0);
}

/// The target must be a creature whose controller controls an Island: with no
/// Island on their side the Ogre is not on the menu (my own Seasinger is).
#[test]
fn seasinger_cannot_target_a_creature_whose_controller_has_no_island() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[seasinger(), island()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let sea = on_battlefield(&engine, P0, seasinger()).expect("the Seasinger is out");
    let ogre = on_battlefield(&engine, P1, gray_ogre()).expect("their Ogre");
    activate(&mut engine, P0, seasinger(), 2);
    let menu = aim_at(&mut engine, P0, sea);
    assert!(menu.contains(&sea), "my own creature: I control an Island");
    assert!(!menu.contains(&ogre), "they control no Island");
}

/// "When you control no Islands, sacrifice this creature."
#[test]
fn seasinger_is_sacrificed_when_its_controller_loses_the_last_island() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[seasinger(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let isle = on_battlefield(&engine, P0, island()).expect("my Island");
    assert!(
        on_battlefield(&engine, P0, seasinger()).is_some(),
        "an Island keeps it"
    );
    kill(&mut engine, isle);
    assert!(
        on_battlefield(&engine, P0, seasinger()).is_none(),
        "no Islands: sacrificed"
    );
    assert!(in_graveyard(&engine, P0, seasinger()).is_some());
}
