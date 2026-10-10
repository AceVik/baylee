//! `cards/artifacts/mv_2/ashnod_s_battle_gear.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ashnod's Battle Gear — {2} artifact: "You may choose not to untap this
/// artifact during your untap step. {2}, {T}: Target creature you control gets
/// +2/-2 for as long as this artifact remains tapped."
fn ashnod_s_battle_gear() -> CardIndex {
    card_index("b5a390fd-2864-4481-84b4-41e8fac91a80")
}

/// Turn one, main phase: the Gear, two Forests and a Hill Giant (3/3) on my
/// side, a Hill Giant across the table. The Gear is activated at mine and
/// resolved; the board is handed back with the Gear and the Giant.
fn pumped() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[ashnod_s_battle_gear(), forest(), forest(), hill_giant()],
        )
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let gear = on_battlefield(&engine, p0, ashnod_s_battle_gear()).expect("the Gear");
    let giant = on_battlefield(&engine, p0, hill_giant()).expect("my Giant");
    assert_eq!(pt(&engine, giant), (3, 3));
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, ashnod_s_battle_gear(), 1);
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);
    (engine, gear, giant)
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

/// The pump is +2/-2 on my creature only, and the Gear is tapped for it.
#[test]
fn ashnod_s_battle_gear_gives_plus_two_minus_two_and_taps() {
    let (engine, gear, giant) = pumped();
    assert!(is_tapped(&engine, gear), "{{T}} was paid");
    assert_eq!(pt(&engine, giant), (5, 1));
}

/// "Target creature you control": the opponent's Giant is not on the menu.
#[test]
fn ashnod_s_battle_gear_offers_only_my_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[ashnod_s_battle_gear(), forest(), forest(), hill_giant()],
        )
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mine = on_battlefield(&engine, p0, hill_giant()).expect("mine");
    let theirs = on_battlefield(&engine, p1, hill_giant()).expect("theirs");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, ashnod_s_battle_gear(), 1);
    let options = aim_at(&mut engine, p0, mine);
    assert!(options.contains(&mine));
    assert!(!options.contains(&theirs), "{options:?}");
}

/// Choosing not to untap the Gear keeps the effect on through the opponent's
/// turn and into my next one (CR 611.2b: it lasts while the Gear stays tapped).
#[test]
fn ashnod_s_battle_gear_lasts_while_i_choose_not_to_untap_it() {
    let p0 = PlayerId::new(0);
    let (mut engine, gear, giant) = pumped();
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert_eq!(pt(&engine, giant), (5, 1), "through their turn");
    let options = to_the_untap_question(&mut engine);
    assert!(options.contains(&gear), "the Gear may stay down");
    assert_eq!(pt(&engine, giant), (5, 1), "still tapped at the question");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![gear],
            },
        )
        .expect("leave it tapped");
    reach_main_phase(&mut engine, p0);
    assert!(is_tapped(&engine, gear));
    assert_eq!(pt(&engine, giant), (5, 1), "into my next turn");
}

/// Untapping the Gear in the untap step ends the effect at once.
#[test]
fn ashnod_s_battle_gear_ends_when_it_untaps() {
    let p0 = PlayerId::new(0);
    let (mut engine, gear, giant) = pumped();
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    to_the_untap_question(&mut engine);
    // `answer_one` untaps everything.
    let (player, action) = answer_one(&engine).expect("the question");
    engine.apply(player, action).expect("legal");
    reach_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, gear), "the Gear untapped");
    assert_eq!(pt(&engine, giant), (3, 3), "the pump is gone");
}
