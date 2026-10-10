//! `cards/artifacts/mv_2/tawnos_s_weaponry.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tawnos's Weaponry — {2} artifact: "You may choose not to untap this
/// artifact during your untap step. {2}, {T}: Target creature gets +1/+1 for
/// as long as this artifact remains tapped."
fn tawnos_s_weaponry() -> CardIndex {
    card_index("f07f98bb-4190-4643-aeb9-c5eaf358c97c")
}

/// Turn one, main phase: Weaponry, two Forests and a Mountain, a Hill Giant on
/// my side with a Shatter in hand, a Hill Giant across the table. The Weaponry
/// is activated at `theirs` (or my own Giant) and resolved.
fn pumped(at_theirs: bool) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                tawnos_s_weaponry(),
                forest(),
                forest(),
                mountain(),
                hill_giant(),
            ],
        )
        .battlefield(1, &[hill_giant()])
        .hand(0, &[shatter()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let weaponry = on_battlefield(&engine, p0, tawnos_s_weaponry()).expect("the Weaponry");
    let mine = on_battlefield(&engine, p0, hill_giant()).expect("my Giant");
    let theirs = on_battlefield(&engine, p1, hill_giant()).expect("their Giant");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, tawnos_s_weaponry(), 1);
    let target = if at_theirs { theirs } else { mine };
    let options = aim_at(&mut engine, p0, target);
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "any creature"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, weaponry), "{{T}} was paid");
    assert_eq!(pt(&engine, target), (4, 4));
    (engine, weaponry, target)
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

/// Leaves the Weaponry tapped through the next untap step and returns to my
/// main phase.
#[track_caller]
fn keep_it_down(engine: &mut Engine<RegistryLookup>, weaponry: ObjectId) {
    let p0 = PlayerId::new(0);
    reach_their_main_phase(engine, PlayerId::new(1));
    let options = to_the_untap_question(engine);
    assert!(options.contains(&weaponry));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![weaponry],
            },
        )
        .expect("leave it tapped");
    reach_main_phase(engine, p0);
    assert!(is_tapped(engine, weaponry));
}

/// +1/+1 on any creature, the opponent's included.
#[test]
fn tawnos_s_weaponry_gives_plus_one_plus_one_to_any_creature() {
    let (engine, _w, target) = pumped(true);
    assert_eq!(pt(&engine, target), (4, 4));
    let (engine, _w, target) = pumped(false);
    assert_eq!(pt(&engine, target), (4, 4));
}

/// It persists across turns while the Weaponry is kept tapped.
#[test]
fn tawnos_s_weaponry_lasts_across_turns_while_it_stays_tapped() {
    let (mut engine, weaponry, target) = pumped(false);
    keep_it_down(&mut engine, weaponry);
    assert_eq!(pt(&engine, target), (4, 4));
}

/// Untapping the Weaponry ends it.
#[test]
fn tawnos_s_weaponry_ends_when_it_untaps() {
    let p0 = PlayerId::new(0);
    let (mut engine, weaponry, target) = pumped(false);
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    to_the_untap_question(&mut engine);
    assert_eq!(pt(&engine, target), (4, 4), "still tapped at the question");
    let (player, action) = answer_one(&engine).expect("the question");
    engine.apply(player, action).expect("legal");
    reach_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, weaponry));
    assert_eq!(pt(&engine, target), (3, 3));
}

/// The Weaponry leaving the battlefield (Shatter) ends it even though nothing
/// untapped.
#[test]
fn tawnos_s_weaponry_ends_when_it_leaves_the_battlefield() {
    let p0 = PlayerId::new(0);
    let (mut engine, weaponry, target) = pumped(false);
    keep_it_down(&mut engine, weaponry);
    assert_eq!(pt(&engine, target), (4, 4));
    cast_from_hand(&mut engine, p0, shatter());
    aim_at(&mut engine, p0, weaponry);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, tawnos_s_weaponry()).is_none());
    assert_eq!(pt(&engine, target), (3, 3));
}
